// SPDX-License-Identifier: MPL-2.0 OR LGPL-3.0-or-later
/*
 * libpathrs: safe path resolution on Linux
 * Copyright (C) 2019-2025 SUSE LLC
 * Copyright (C) 2026 Aleksa Sarai <cyphar@cyphar.com>
 *
 * == MPL-2.0 ==
 *
 *  This Source Code Form is subject to the terms of the Mozilla Public
 *  License, v. 2.0. If a copy of the MPL was not distributed with this
 *  file, You can obtain one at https://mozilla.org/MPL/2.0/.
 *
 * Alternatively, this Source Code Form may also (at your option) be used
 * under the terms of the GNU Lesser General Public License Version 3, as
 * described below:
 *
 * == LGPL-3.0-or-later ==
 *
 *  This program is free software: you can redistribute it and/or modify it
 *  under the terms of the GNU Lesser General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or (at
 *  your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful, but
 *  WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY  or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General
 * Public License  for more details.
 *
 *  You should have received a copy of the GNU Lesser General Public License
 *  along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

//! libpathrs provides a series of primitives for Linux programs to safely
//! handle path operations inside an untrusted directory tree. [There are
//! countless examples of security vulnerabilities caused by bad handling of
//! paths][avoidable-issues]; this library provides an easy-to-use set of VFS
//! APIs to avoid those kinds of issues.
//!
//! The idea is that a [`Root`] handle is like a handle for resolution inside a
//! [`chroot(2)`], with [`Handle`] being an `O_PATH` descriptor which you can
//! "upgrade" to a proper [`File`]. However this library acts far more
//! efficiently than spawning a new process and doing a full [`chroot(2)`] for
//! every operation.
//!
//! [avoidable-issues]: https://github.com/cyphar/libpathrs/blob/main/docs/avoidable-vulnerabilities.md
//! [`chroot(2)`]: http://man7.org/linux/man-pages/man2/chroot.2.html
//! [`File`]: std::fs::File
//! [`ProcfsHandle`]: crate::procfs::ProcfsHandle
//!
//! # Example
//!
//! The recommended usage of libpathrs looks something like this:
//!
//! ```
//! # use pathrs::{error::Error, flags::OpenFlags, Root};
//! # fn main() -> Result<(), Error> {
//! let (root_path, unsafe_path) = ("/path/to/root", "/etc/passwd");
//! # let root_path = "/";
//! // Get a root handle for resolution.
//! let root = Root::open(root_path)?;
//! // Resolve the path.
//! let handle = root.resolve(unsafe_path)?;
//! // Upgrade the handle to a full std::fs::File.
//! let file = handle.reopen(OpenFlags::O_RDONLY)?;
//!
//! // Or, in one line:
//! let file = root.resolve(unsafe_path)?
//!                .reopen(OpenFlags::O_RDONLY)?;
//! # Ok(())
//! # }
//! ```
//!
//! # Kernel Support
//!
//! At the moment, `libpathrs` only works on Linux as it was designed around
//! Linux-only APIs that are necessary to provide safe path operations. In
//! future, we plan to expand support for other Unix-like operating systems.
//!
//! Please consult the [markdown documentation][kernel-feature-list] for the
//! latest information about what kernel features are supported and recommended
//! minimum kernel versions.
//!
//! [kernel-feature-list]: https://github.com/cyphar/libpathrs/blob/main/docs/kernel-features.md

// libpathrs only supports Linux at the moment.
#![cfg(target_os = "linux")]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(clippy::all)]
#![deny(missing_debug_implementations)]
// We use this the coverage_attribute when doing coverage runs.
// <https://github.com/rust-lang/rust/issues/84605>
#![cfg_attr(coverage, feature(coverage_attribute))]

// `Handle` implementation.
mod handle;
#[doc(inline)]
pub use handle::*;

// `Root` implementation.
mod root;
#[doc(inline)]
pub use root::*;

pub mod error;
pub mod flags;
pub mod procfs;

// Resolver backend implementations.
mod resolvers;

// C API.
#[cfg(feature = "capi")]
mod capi;

// Internally used helpers.
mod syscalls;
mod utils;

// Library tests.
#[cfg(test)]
mod tests;
