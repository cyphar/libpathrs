// SPDX-License-Identifier: MPL-2.0 OR LGPL-3.0-or-later
/*
 * libpathrs: safe path resolution on Linux
 * Copyright (C) 2019-2025 Aleksa Sarai <cyphar@cyphar.com>
 * Copyright (C) 2019-2025 SUSE LLC
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
    error::Error,
    flags::OpenFlags,
    procfs::{ProcfsBase, ProcfsHandleRef},
    tests::traits::ErrorImpl,
};

use std::{
    fs::File,
    path::{Path, PathBuf},
};

pub(in crate::tests) trait ProcfsHandleImpl: std::fmt::Debug {
    type Error: ErrorImpl;

    fn open_follow(
        &self,
        base: ProcfsBase,
        subpath: impl AsRef<Path>,
        flags: impl Into<OpenFlags>,
    ) -> Result<File, Self::Error>;

    fn open(
        &self,
        base: ProcfsBase,
        subpath: impl AsRef<Path>,
        flags: impl Into<OpenFlags>,
    ) -> Result<File, Self::Error>;

    fn readlink(&self, base: ProcfsBase, subpath: impl AsRef<Path>)
        -> Result<PathBuf, Self::Error>;
}

impl<'fd> ProcfsHandleImpl for ProcfsHandleRef<'fd> {
    type Error = Error;

    fn open_follow(
        &self,
        base: ProcfsBase,
        subpath: impl AsRef<Path>,
        flags: impl Into<OpenFlags>,
    ) -> Result<File, Self::Error> {
        self.open_follow(base, subpath, flags)
    }

    fn open(
        &self,
        base: ProcfsBase,
        subpath: impl AsRef<Path>,
        flags: impl Into<OpenFlags>,
    ) -> Result<File, Self::Error> {
        self.open(base, subpath, flags)
    }

    fn readlink(
        &self,
        base: ProcfsBase,
        subpath: impl AsRef<Path>,
    ) -> Result<PathBuf, Self::Error> {
        self.readlink(base, subpath)
    }
}
