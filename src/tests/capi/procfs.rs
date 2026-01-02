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
    capi::{
        self,
        procfs::{CProcfsBase, ProcfsOpenFlags, ProcfsOpenHow},
    },
    flags::OpenFlags,
    procfs::ProcfsBase,
    tests::{
        capi::utils::{self as capi_utils, CapiError},
        traits::ProcfsHandleImpl,
    },
};

use std::{
    fs::File,
    mem,
    os::unix::io::{AsFd, OwnedFd},
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct CapiProcfsHandle;

impl CapiProcfsHandle {
    fn open_follow(
        &self,
        base: ProcfsBase,
        subpath: impl AsRef<Path>,
        oflags: impl Into<OpenFlags>,
    ) -> Result<File, CapiError> {
        let base: CProcfsBase = base.into();
        let subpath = capi_utils::path_to_cstring(subpath);
        let oflags = oflags.into();

        capi_utils::call_capi_fd(|| unsafe {
            capi::procfs::pathrs_proc_open(base, subpath.as_ptr(), oflags.bits())
        })
        .map(File::from)
    }

    fn open(
        &self,
        base: ProcfsBase,
        subpath: impl AsRef<Path>,
        oflags: impl Into<OpenFlags>,
    ) -> Result<File, CapiError> {
        // The C API exposes ProcfsHandle::open using O_NOFOLLOW.
        self.open_follow(base, subpath, oflags.into() | OpenFlags::O_NOFOLLOW)
    }

    fn readlink(&self, base: ProcfsBase, subpath: impl AsRef<Path>) -> Result<PathBuf, CapiError> {
        let base: CProcfsBase = base.into();
        let subpath = capi_utils::path_to_cstring(subpath);

        capi_utils::call_capi_readlink(|linkbuf, linkbuf_size| unsafe {
            capi::procfs::pathrs_proc_readlink(base, subpath.as_ptr(), linkbuf, linkbuf_size)
        })
    }
}

impl ProcfsHandleImpl for CapiProcfsHandle {
    type Error = CapiError;

    fn open_follow(
        &self,
        base: ProcfsBase,
        subpath: impl AsRef<Path>,
        oflags: impl Into<OpenFlags>,
    ) -> Result<File, Self::Error> {
        self.open_follow(base, subpath, oflags)
    }

    fn open(
        &self,
        base: ProcfsBase,
        subpath: impl AsRef<Path>,
        oflags: impl Into<OpenFlags>,
    ) -> Result<File, Self::Error> {
        self.open(base, subpath, oflags)
    }

    fn readlink(
        &self,
        base: ProcfsBase,
        subpath: impl AsRef<Path>,
    ) -> Result<PathBuf, Self::Error> {
        self.readlink(base, subpath)
    }
}

#[derive(Debug)]
pub struct CapiProcfsHandleFd(pub OwnedFd);

impl From<CapiProcfsHandleFd> for OwnedFd {
    fn from(procfs: CapiProcfsHandleFd) -> Self {
        procfs.0
    }
}

impl CapiProcfsHandleFd {
    pub fn new_unmasked() -> Result<Self, CapiError> {
        capi_utils::call_capi_fd(|| unsafe {
            let how = ProcfsOpenHow {
                flags: ProcfsOpenFlags::PATHRS_PROCFS_NEW_UNMASKED,
            };

            capi::procfs::pathrs_procfs_open(&how as *const _, mem::size_of::<ProcfsOpenHow>())
        })
        .map(CapiProcfsHandleFd)
    }

    fn open_follow(
        &self,
        base: ProcfsBase,
        subpath: impl AsRef<Path>,
        oflags: impl Into<OpenFlags>,
    ) -> Result<File, CapiError> {
        let base: CProcfsBase = base.into();
        let subpath = capi_utils::path_to_cstring(subpath);
        let oflags = oflags.into();

        capi_utils::call_capi_fd(|| unsafe {
            capi::procfs::pathrs_proc_openat(
                self.0.as_fd().into(),
                base,
                subpath.as_ptr(),
                oflags.bits(),
            )
        })
        .map(File::from)
    }

    fn open(
        &self,
        base: ProcfsBase,
        subpath: impl AsRef<Path>,
        oflags: impl Into<OpenFlags>,
    ) -> Result<File, CapiError> {
        // The C API exposes ProcfsHandle::open using O_NOFOLLOW.
        self.open_follow(base, subpath, oflags.into() | OpenFlags::O_NOFOLLOW)
    }

    fn readlink(&self, base: ProcfsBase, subpath: impl AsRef<Path>) -> Result<PathBuf, CapiError> {
        let base: CProcfsBase = base.into();
        let subpath = capi_utils::path_to_cstring(subpath);

        capi_utils::call_capi_readlink(|linkbuf, linkbuf_size| unsafe {
            capi::procfs::pathrs_proc_readlinkat(
                self.0.as_fd().into(),
                base,
                subpath.as_ptr(),
                linkbuf,
                linkbuf_size,
            )
        })
    }
}

impl ProcfsHandleImpl for CapiProcfsHandleFd {
    type Error = CapiError;

    fn open_follow(
        &self,
        base: ProcfsBase,
        subpath: impl AsRef<Path>,
        oflags: impl Into<OpenFlags>,
    ) -> Result<File, Self::Error> {
        self.open_follow(base, subpath, oflags)
    }

    fn open(
        &self,
        base: ProcfsBase,
        subpath: impl AsRef<Path>,
        oflags: impl Into<OpenFlags>,
    ) -> Result<File, Self::Error> {
        self.open(base, subpath, oflags)
    }

    fn readlink(
        &self,
        base: ProcfsBase,
        subpath: impl AsRef<Path>,
    ) -> Result<PathBuf, Self::Error> {
        self.readlink(base, subpath)
    }
}
