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

use crate::{
    error::{Error, ErrorImpl},
    flags::{OpenFlags, ResolverFlags},
    resolvers::PartialLookup,
    syscalls::{self, OpenHow},
    utils::PathIterExt,
    Handle,
};

use std::{
    fs::File,
    os::unix::io::AsFd,
    path::{Path, PathBuf},
};

/// Open `path` within `root` through `openat(2)`.
///
/// This is an optimised version of `resolve(root, path, ...)?.reopen(flags)`.
pub(crate) fn open(
    root: impl AsFd,
    path: impl AsRef<Path>,
    rflags: ResolverFlags,
    oflags: OpenFlags,
) -> Result<File, Error> {
    let rflags = libc::RESOLVE_IN_ROOT | libc::RESOLVE_NO_MAGICLINKS | rflags.bits();
    let how = OpenHow {
        flags: oflags.bits() as u64,
        resolve: rflags,
        ..Default::default()
    };

    syscalls::openat2_follow(&root, path.as_ref(), how)
        .map(File::from)
        .map_err(|err| {
            ErrorImpl::RawOsError {
                operation: "openat2 one-shot open".into(),
                source: err,
            }
            .into()
        })
}

/// Resolve `path` within `root` through `openat2(2)`.
pub(crate) fn resolve(
    root: impl AsFd,
    path: impl AsRef<Path>,
    rflags: ResolverFlags,
    no_follow_trailing: bool,
) -> Result<Handle, Error> {
    // Copy the O_NOFOLLOW and RESOLVE_NO_SYMLINKS bits from flags.
    let mut oflags = OpenFlags::O_PATH;
    if no_follow_trailing {
        oflags.insert(OpenFlags::O_NOFOLLOW);
    }
    let rflags = libc::RESOLVE_IN_ROOT | libc::RESOLVE_NO_MAGICLINKS | rflags.bits();

    let how = OpenHow {
        flags: oflags.bits() as u64,
        resolve: rflags,
        ..Default::default()
    };

    syscalls::openat2_follow(&root, path.as_ref(), how)
        .map(Handle::from_fd)
        .map_err(|err| {
            ErrorImpl::RawOsError {
                operation: "openat2 subpath".into(),
                source: err,
            }
            .into()
        })
}

/// Resolve as many components as possible in `path` within `root` using
/// `openat2(2)`.
pub(crate) fn resolve_partial(
    root: impl AsFd,
    path: impl AsRef<Path>,
    rflags: ResolverFlags,
    no_follow_trailing: bool,
) -> Result<PartialLookup<Handle>, Error> {
    let root = root.as_fd();
    let path = path.as_ref();

    let mut last_error = match resolve(root, path, rflags, no_follow_trailing) {
        Ok(handle) => return Ok(PartialLookup::Complete(handle)),
        Err(err) => err,
    };

    // TODO: We probably want to do a git-bisect-like binary-search here. For
    //       paths with a large number of components this could make a
    //       significant difference, though in practice you'll only see fairly
    //       short paths so the implementation complexity might not be worth it.
    for (path, remaining) in path.partial_ancestors() {
        if last_error.is_safety_violation() {
            // If we hit a safety violation, we return an error instead of a
            // partial resolution to match the behaviour of the O_PATH
            // resolver (and to avoid some possible weird bug in libpathrs
            // being exploited to return some result to Root::mkdir_all).
            return Err(last_error);
        }
        match resolve(root, path, rflags, no_follow_trailing) {
            Ok(handle) => {
                return Ok(PartialLookup::Partial {
                    handle,
                    remaining: remaining.map(PathBuf::from).unwrap_or("".into()),
                    last_error,
                })
            }
            Err(err) => last_error = err,
        }
    }

    Err(last_error)
}
