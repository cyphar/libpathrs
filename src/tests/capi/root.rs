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
    capi,
    flags::{OpenFlags, RenameFlags},
    resolvers::Resolver,
    tests::{
        capi::{
            utils::{self as capi_utils, CapiError},
            CapiHandle,
        },
        traits::{HandleImpl, RootImpl},
    },
    InodeType,
};

use std::{
    fs::{File, Permissions},
    os::unix::{
        fs::PermissionsExt,
        io::{AsFd, BorrowedFd, OwnedFd},
    },
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub(in crate::tests) struct CapiRoot {
    inner: OwnedFd,
}

impl CapiRoot {
    pub(in crate::tests) fn open(path: impl AsRef<Path>) -> Result<Self, CapiError> {
        let path = capi_utils::path_to_cstring(path);

        capi_utils::call_capi_fd(|| unsafe { capi::core::pathrs_open_root(path.as_ptr()) })
            .map(Self::from_fd)
    }

    pub(in crate::tests) fn from_fd(fd: impl Into<OwnedFd>) -> Self {
        Self { inner: fd.into() }
    }

    fn try_clone(&self) -> Result<Self, anyhow::Error> {
        Ok(Self::from_fd(self.inner.try_clone()?))
    }

    fn resolve(&self, path: impl AsRef<Path>) -> Result<CapiHandle, CapiError> {
        let root_fd = self.inner.as_fd();
        let path = capi_utils::path_to_cstring(path);

        capi_utils::call_capi_fd(|| unsafe {
            capi::core::pathrs_inroot_resolve(root_fd.into(), path.as_ptr())
        })
        .map(CapiHandle::from_fd)
    }

    fn resolve_nofollow(&self, path: impl AsRef<Path>) -> Result<CapiHandle, CapiError> {
        let root_fd = self.inner.as_fd();
        let path = capi_utils::path_to_cstring(path);

        capi_utils::call_capi_fd(|| unsafe {
            capi::core::pathrs_inroot_resolve_nofollow(root_fd.into(), path.as_ptr())
        })
        .map(CapiHandle::from_fd)
    }

    fn open_subpath(
        &self,
        path: impl AsRef<Path>,
        flags: impl Into<OpenFlags>,
    ) -> Result<File, CapiError> {
        let root_fd = self.inner.as_fd();
        let path = capi_utils::path_to_cstring(path);
        let flags = flags.into();

        capi_utils::call_capi_fd(|| unsafe {
            capi::core::pathrs_inroot_open(root_fd.into(), path.as_ptr(), flags.bits())
        })
        .map(From::from)
    }

    fn readlink(&self, path: impl AsRef<Path>) -> Result<PathBuf, CapiError> {
        let root_fd = self.inner.as_fd();
        let path = capi_utils::path_to_cstring(path);

        capi_utils::call_capi_readlink(|linkbuf, linkbuf_size| unsafe {
            capi::core::pathrs_inroot_readlink(root_fd.into(), path.as_ptr(), linkbuf, linkbuf_size)
        })
    }

    fn create(&self, path: impl AsRef<Path>, inode_type: &InodeType) -> Result<(), CapiError> {
        let root_fd = self.inner.as_fd();
        let path = capi_utils::path_to_cstring(path);

        capi_utils::call_capi_zst(|| match inode_type {
            InodeType::File(perm) => unsafe {
                capi::core::pathrs_inroot_mknod(
                    root_fd.into(),
                    path.as_ptr(),
                    libc::S_IFREG | perm.mode(),
                    0,
                )
            },
            InodeType::Directory(perm) => unsafe {
                capi::core::pathrs_inroot_mkdir(root_fd.into(), path.as_ptr(), perm.mode())
            },
            InodeType::Symlink(target) => {
                let target = capi_utils::path_to_cstring(target);
                unsafe {
                    capi::core::pathrs_inroot_symlink(
                        root_fd.into(),
                        path.as_ptr(),
                        target.as_ptr(),
                    )
                }
            }
            InodeType::Hardlink(target) => {
                let target = capi_utils::path_to_cstring(target);
                unsafe {
                    capi::core::pathrs_inroot_hardlink(
                        root_fd.into(),
                        path.as_ptr(),
                        target.as_ptr(),
                    )
                }
            }
            InodeType::Fifo(perm) => unsafe {
                capi::core::pathrs_inroot_mknod(
                    root_fd.into(),
                    path.as_ptr(),
                    libc::S_IFIFO | perm.mode(),
                    0,
                )
            },
            InodeType::CharacterDevice(perm, dev) => unsafe {
                capi::core::pathrs_inroot_mknod(
                    root_fd.into(),
                    path.as_ptr(),
                    libc::S_IFCHR | perm.mode(),
                    *dev,
                )
            },
            InodeType::BlockDevice(perm, dev) => unsafe {
                capi::core::pathrs_inroot_mknod(
                    root_fd.into(),
                    path.as_ptr(),
                    libc::S_IFBLK | perm.mode(),
                    *dev,
                )
            },
        })
    }

    fn create_file(
        &self,
        path: impl AsRef<Path>,
        flags: impl Into<OpenFlags>,
        perm: &Permissions,
    ) -> Result<File, CapiError> {
        let root_fd = self.inner.as_fd();
        let path = capi_utils::path_to_cstring(path);
        let flags = flags.into();

        capi_utils::call_capi_fd(|| unsafe {
            capi::core::pathrs_inroot_creat(
                root_fd.into(),
                path.as_ptr(),
                flags.bits(),
                perm.mode(),
            )
        })
        .map(File::from)
    }

    fn mkdir_all(
        &self,
        path: impl AsRef<Path>,
        perm: &Permissions,
    ) -> Result<CapiHandle, CapiError> {
        let root_fd = self.inner.as_fd();
        let path = capi_utils::path_to_cstring(path);

        capi_utils::call_capi_fd(|| unsafe {
            capi::core::pathrs_inroot_mkdir_all(root_fd.into(), path.as_ptr(), perm.mode())
        })
        .map(CapiHandle::from_fd)
    }

    fn remove_dir(&self, path: impl AsRef<Path>) -> Result<(), CapiError> {
        let root_fd = self.inner.as_fd();
        let path = capi_utils::path_to_cstring(path);

        capi_utils::call_capi_zst(|| unsafe {
            capi::core::pathrs_inroot_rmdir(root_fd.into(), path.as_ptr())
        })
    }

    fn remove_file(&self, path: impl AsRef<Path>) -> Result<(), CapiError> {
        let root_fd = self.inner.as_fd();
        let path = capi_utils::path_to_cstring(path);

        capi_utils::call_capi_zst(|| unsafe {
            capi::core::pathrs_inroot_unlink(root_fd.into(), path.as_ptr())
        })
    }

    fn remove_all(&self, path: impl AsRef<Path>) -> Result<(), CapiError> {
        let root_fd = self.inner.as_fd();
        let path = capi_utils::path_to_cstring(path);

        capi_utils::call_capi_zst(|| unsafe {
            capi::core::pathrs_inroot_remove_all(root_fd.into(), path.as_ptr())
        })
    }

    fn rename(
        &self,
        source: impl AsRef<Path>,
        destination: impl AsRef<Path>,
        rflags: RenameFlags,
    ) -> Result<(), CapiError> {
        let root_fd = self.inner.as_fd();
        let source = capi_utils::path_to_cstring(source);
        let destination = capi_utils::path_to_cstring(destination);

        capi_utils::call_capi_zst(|| unsafe {
            capi::core::pathrs_inroot_rename(
                root_fd.into(),
                source.as_ptr(),
                destination.as_ptr(),
                rflags.bits(),
            )
        })
    }
}

impl AsFd for CapiRoot {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.inner.as_fd()
    }
}

impl From<CapiRoot> for OwnedFd {
    fn from(root: CapiRoot) -> Self {
        root.inner
    }
}

impl RootImpl for CapiRoot {
    type Cloned = CapiRoot;
    type Handle = CapiHandle;
    // NOTE: We can't use anyhow::Error here.
    // <https://github.com/dtolnay/anyhow/issues/25>
    type Error = CapiError;

    fn from_fd(fd: impl Into<OwnedFd>, resolver: Resolver) -> Self::Cloned {
        assert_eq!(
            resolver,
            Resolver::default(),
            "cannot use non-default Resolver with capi"
        );
        Self::Cloned::from_fd(fd)
    }

    fn resolver(&self) -> Resolver {
        Resolver::default()
    }

    fn try_clone(&self) -> Result<Self::Cloned, anyhow::Error> {
        self.try_clone()
    }

    fn resolve(&self, path: impl AsRef<Path>) -> Result<Self::Handle, Self::Error> {
        self.resolve(path)
    }

    fn resolve_nofollow(&self, path: impl AsRef<Path>) -> Result<Self::Handle, Self::Error> {
        self.resolve_nofollow(path)
    }

    fn open_subpath(
        &self,
        path: impl AsRef<Path>,
        flags: impl Into<OpenFlags>,
    ) -> Result<File, Self::Error> {
        self.open_subpath(path, flags)
    }

    fn readlink(&self, path: impl AsRef<Path>) -> Result<PathBuf, Self::Error> {
        self.readlink(path)
    }

    fn create(&self, path: impl AsRef<Path>, inode_type: &InodeType) -> Result<(), Self::Error> {
        self.create(path, inode_type)
    }

    fn create_file(
        &self,
        path: impl AsRef<Path>,
        flags: OpenFlags,
        perm: &Permissions,
    ) -> Result<File, Self::Error> {
        self.create_file(path, flags, perm)
    }

    fn mkdir_all(
        &self,
        path: impl AsRef<Path>,
        perm: &Permissions,
    ) -> Result<Self::Handle, Self::Error> {
        self.mkdir_all(path, perm)
    }

    fn remove_dir(&self, path: impl AsRef<Path>) -> Result<(), Self::Error> {
        self.remove_dir(path)
    }

    fn remove_file(&self, path: impl AsRef<Path>) -> Result<(), Self::Error> {
        self.remove_file(path)
    }

    fn remove_all(&self, path: impl AsRef<Path>) -> Result<(), Self::Error> {
        self.remove_all(path)
    }

    fn rename(
        &self,
        source: impl AsRef<Path>,
        destination: impl AsRef<Path>,
        rflags: RenameFlags,
    ) -> Result<(), CapiError> {
        self.rename(source, destination, rflags)
    }
}

impl RootImpl for &CapiRoot {
    type Cloned = CapiRoot;
    type Handle = CapiHandle;
    // NOTE: We can't use anyhow::Error here.
    // <https://github.com/dtolnay/anyhow/issues/25>
    type Error = CapiError;

    fn from_fd(fd: impl Into<OwnedFd>, resolver: Resolver) -> Self::Cloned {
        assert_eq!(
            resolver,
            Resolver::default(),
            "cannot use non-default Resolver with capi"
        );
        Self::Cloned::from_fd(fd)
    }

    fn resolver(&self) -> Resolver {
        Resolver::default()
    }

    fn try_clone(&self) -> Result<Self::Cloned, anyhow::Error> {
        CapiRoot::try_clone(self)
    }

    fn resolve(&self, path: impl AsRef<Path>) -> Result<Self::Handle, Self::Error> {
        CapiRoot::resolve(self, path)
    }

    fn resolve_nofollow(&self, path: impl AsRef<Path>) -> Result<Self::Handle, Self::Error> {
        CapiRoot::resolve_nofollow(self, path)
    }

    fn open_subpath(
        &self,
        path: impl AsRef<Path>,
        flags: impl Into<OpenFlags>,
    ) -> Result<File, Self::Error> {
        CapiRoot::open_subpath(self, path, flags)
    }

    fn readlink(&self, path: impl AsRef<Path>) -> Result<PathBuf, Self::Error> {
        CapiRoot::readlink(self, path)
    }

    fn create(&self, path: impl AsRef<Path>, inode_type: &InodeType) -> Result<(), Self::Error> {
        CapiRoot::create(self, path, inode_type)
    }

    fn create_file(
        &self,
        path: impl AsRef<Path>,
        flags: OpenFlags,
        perm: &Permissions,
    ) -> Result<File, Self::Error> {
        CapiRoot::create_file(self, path, flags, perm)
    }

    fn mkdir_all(
        &self,
        path: impl AsRef<Path>,
        perm: &Permissions,
    ) -> Result<Self::Handle, Self::Error> {
        CapiRoot::mkdir_all(self, path, perm)
    }

    fn remove_dir(&self, path: impl AsRef<Path>) -> Result<(), Self::Error> {
        CapiRoot::remove_dir(self, path)
    }

    fn remove_file(&self, path: impl AsRef<Path>) -> Result<(), Self::Error> {
        CapiRoot::remove_file(self, path)
    }

    fn remove_all(&self, path: impl AsRef<Path>) -> Result<(), Self::Error> {
        CapiRoot::remove_all(self, path)
    }

    fn rename(
        &self,
        source: impl AsRef<Path>,
        destination: impl AsRef<Path>,
        rflags: RenameFlags,
    ) -> Result<(), CapiError> {
        CapiRoot::rename(self, source, destination, rflags)
    }
}
