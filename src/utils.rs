// SPDX-License-Identifier: LGPL-3.0-or-later
/*
 * libpathrs: safe path resolution on Linux
 * Copyright (C) 2019-2025 Aleksa Sarai <cyphar@cyphar.com>
 * Copyright (C) 2019-2025 SUSE LLC
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

#![forbid(unsafe_code)]

mod dir;
pub(crate) use dir::*;

mod path;
pub(crate) use path::*;

mod fd;
pub(crate) use fd::*;

mod fdinfo;
pub(crate) use fdinfo::*;

mod sysctl;
pub(crate) use sysctl::*;

mod maybe_owned;
pub(crate) use maybe_owned::*;

mod raw_procfs;
pub(crate) use raw_procfs::*;
