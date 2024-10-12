/*
 * libpathrs: safe path resolution on Linux
 * Copyright (C) 2019-2024 Aleksa Sarai <cyphar@cyphar.com>
 * Copyright (C) 2019-2024 SUSE LLC
 *
 * This program is free software: you can redistribute it and/or modify it
 * under the terms of the GNU Lesser General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or (at your
 * option) any later version.
 *
 * This program is distributed in the hope that it will be useful, but
 * WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY
 * or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License
 * for more details.
 *
 * You should have received a copy of the GNU Lesser General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

// We need to permit unsafe code because we are exposing C APIs over FFI and
// thus need to interact with C callers.
#![allow(unsafe_code)]

/// Core pathrs function wrappers.
pub mod core;

/// Helpers for pathrs C API configuration.
pub mod cfg;

/// procfs-related function wrappers.
pub mod procfs;

/// C-friendly [`Error`](crate::error::Error) representation and helpers.
pub mod error;

/// Helpers for converting [`Result`] into C-style int returns.
pub mod ret;

mod utils;
