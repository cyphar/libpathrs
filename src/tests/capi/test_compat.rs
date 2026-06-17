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
    capi,
    error::ErrorKind,
    flags::OpenFlags,
    procfs::ProcfsHandle,
    tests::{capi::utils as capi_utils, common as tests_common, traits::ErrorImpl},
    utils::FdExt,
    Root,
};

use std::{
    fs::File,
    os::unix::{
        fs::MetadataExt,
        io::{AsFd, AsRawFd, OwnedFd},
    },
};

use anyhow::{Context, Error};
use pretty_assertions::{assert_eq, assert_matches};

#[test]
fn reopen_v1() -> Result<(), Error> {
    let file: OwnedFd = File::open(".").context("open dummy file")?.into();

    let oflags = OpenFlags::O_DIRECTORY | OpenFlags::O_RDONLY | OpenFlags::O_NOCTTY;
    let reopened_fd = capi_utils::call_capi_fd(|| {
        capi::core::__pathrs_reopen_v1(file.as_fd().into(), oflags.bits() as i32)
    })
    .with_context(|| format!("__pathrs_reopen_v1({oflags:?})"))?;

    assert_ne!(
        file.as_raw_fd(),
        reopened_fd.as_raw_fd(),
        "new and reopened fds should have different fd numbers"
    );
    assert_eq!(
        file.as_unsafe_path_unchecked()
            .expect("get real path of original fd"),
        reopened_fd
            .as_unsafe_path_unchecked()
            .expect("get real path of reopened fd"),
        "new and reopened fds should have the same 'real' path",
    );
    tests_common::check_oflags(&reopened_fd, oflags).expect("check reopened fd flags");

    Ok(())
}

#[test]
fn inroot_open_v1() -> Result<(), Error> {
    let root_dir = tests_common::create_basic_tree().context("create basic tree")?;
    let root = Root::open(root_dir.path()).context("Root::open basic tree")?;

    {
        let path = capi_utils::path_to_cstring("b/c");
        let oflags = OpenFlags::O_DIRECTORY | OpenFlags::O_RDONLY | OpenFlags::O_NOCTTY;
        // SAFETY: Called with valid C-like arguments.
        let file = capi_utils::call_capi_fd(|| unsafe {
            capi::core::__pathrs_inroot_open_v1(
                root.as_fd().into(),
                path.as_ptr(),
                oflags.bits() as _,
            )
        })
        .with_context(|| format!("__pathrs_inroot_open_v1({path:?}, {oflags:?})"))?;
        tests_common::check_oflags(&file, oflags)
            .with_context(|| format!("check {path:?} {oflags:?} oflags"))?;
    }

    {
        let path = capi_utils::path_to_cstring("b/c/file");
        let oflags = OpenFlags::O_RDWR | OpenFlags::O_TRUNC | OpenFlags::O_DIRECT;
        // SAFETY: Called with valid C-like arguments.
        let file = capi_utils::call_capi_fd(|| unsafe {
            capi::core::__pathrs_inroot_open_v1(
                root.as_fd().into(),
                path.as_ptr(),
                oflags.bits() as _,
            )
        })
        .with_context(|| format!("__pathrs_inroot_open_v1({path:?}, {oflags:?})"))?;
        tests_common::check_oflags(&file, oflags)
            .with_context(|| format!("check {path:?} {oflags:?} oflags"))?;
    }

    {
        let path = capi_utils::path_to_cstring("b-file");
        let oflags = OpenFlags::O_NOFOLLOW | OpenFlags::O_PATH;
        // SAFETY: Called with valid C-like arguments.
        let file = capi_utils::call_capi_fd(|| unsafe {
            capi::core::__pathrs_inroot_open_v1(
                root.as_fd().into(),
                path.as_ptr(),
                oflags.bits() as _,
            )
        })
        .with_context(|| format!("__pathrs_inroot_open_v1({path:?}, {oflags:?})"))?;
        tests_common::check_oflags(&file, oflags)
            .with_context(|| format!("check {path:?} {oflags:?} oflags"))?;
    }

    Ok(())
}

#[test]
fn inroot_creat_v1() -> Result<(), Error> {
    let root_dir = tests_common::create_basic_tree().context("create basic tree")?;
    let root = Root::open(root_dir.path()).context("Root::open basic tree")?;

    {
        let path = capi_utils::path_to_cstring("b/c/new-file");
        let oflags = OpenFlags::O_RDWR | OpenFlags::O_NOCTTY | OpenFlags::O_EXCL;
        let mode = 0o644;
        // SAFETY: Called with valid C-like arguments.
        let file = capi_utils::call_capi_fd(|| unsafe {
            capi::core::__pathrs_inroot_creat_v1(
                root.as_fd().into(),
                path.as_ptr(),
                oflags.bits() as _,
                mode,
            )
        })
        .with_context(|| format!("__pathrs_inroot_creat_v1({path:?}, {oflags:?}, 0o{mode:o})"))?;
        tests_common::check_mode(&file, libc::S_IFREG | mode)
            .with_context(|| format!("check created {path:?} file mode 0o{mode:o}"))?;
        tests_common::check_oflags(&file, oflags)
            .with_context(|| format!("check created {path:?} {oflags:?} oflags"))?;
    }

    Ok(())
}

#[test]
fn hardlink_v1() -> Result<(), Error> {
    let root_dir = tests_common::create_basic_tree().context("create basic tree")?;
    let root = Root::open(root_dir.path()).context("Root::open basic tree")?;
    let path1 = capi_utils::path_to_cstring("abc");
    let path2 = capi_utils::path_to_cstring("b/c/file");

    assert_eq!(
        // SAFETY: Called with valid C-like arguments.
        capi_utils::call_capi(|| unsafe {
            capi::core::__pathrs_inroot_hardlink_v1(
                root.as_fd().into(),
                path1.as_ptr(),
                path2.as_ptr(),
            )
        })
        .map_err(|err| err.kind()),
        Ok(0),
        "__pathrs_inroot_hardlink_v1 should work"
    );

    let old_meta = root
        .resolve_nofollow("b/c/file")
        .expect("resolve b/c/file")
        .metadata()
        .expect("fstat b/c/file");
    let new_meta = root
        .resolve_nofollow("abc")
        .expect("hardlink abc should've been created")
        .metadata()
        .expect("fstat hardlink abc");
    assert_eq!(
        old_meta.ino(),
        new_meta.ino(),
        "inode numbers of hardlinks with __pathrs_inroot_hardlink_v1 should be the same",
    );

    Ok(())
}

#[test]
fn hardlink_v2() -> Result<(), Error> {
    let root_dir = tests_common::create_basic_tree().context("create basic tree")?;
    let root1 = Root::open(root_dir.path()).context("Root::open basic tree (#1)")?;
    let root2 = Root::open(root_dir.path()).context("Root::open basic tree (#2)")?;
    let path1 = capi_utils::path_to_cstring("abc");
    let path2 = capi_utils::path_to_cstring("b/c/file");

    assert_eq!(
        // SAFETY: Called with valid C-like arguments.
        capi_utils::call_capi(|| unsafe {
            capi::core::pathrs_inroot_hardlink(
                root1.as_fd().into(),
                path1.as_ptr(),
                root2.as_fd().into(),
                path2.as_ptr(),
                0,
            )
        })
        .map_err(|err| err.kind().errno()),
        Err(ErrorKind::InvalidArgument.errno()),
        "pathrs_inroot_hardlink@LIBPATHRS_0.2.5 should reject old_root_fd != new_root_fd"
    );

    assert_eq!(
        // SAFETY: Called with valid C-like arguments.
        capi_utils::call_capi(|| unsafe {
            capi::core::pathrs_inroot_hardlink(
                root1.as_fd().into(),
                path1.as_ptr(),
                root1.as_fd().into(), // *not* root2!
                path2.as_ptr(),
                0xFFFF,
            )
        })
        .map_err(|err| err.kind().errno()),
        Err(ErrorKind::InvalidArgument.errno()),
        "pathrs_inroot_hardlink@LIBPATHRS_0.2.5 should reject flags != 0"
    );

    Ok(())
}

#[test]
fn symlink_v1() -> Result<(), Error> {
    let root_dir = tests_common::create_basic_tree().context("create basic tree")?;
    let root = Root::open(root_dir.path()).context("Root::open basic tree")?;
    let path1 = capi_utils::path_to_cstring("abc");
    let path2 = capi_utils::path_to_cstring("b/c/file");

    assert_eq!(
        // SAFETY: Called with valid C-like arguments.
        capi_utils::call_capi(|| unsafe {
            capi::core::__pathrs_inroot_symlink_v1(
                root.as_fd().into(),
                path1.as_ptr(),
                path2.as_ptr(),
            )
        })
        .map_err(|err| err.kind()),
        Ok(0),
        "__pathrs_inroot_symlink_v1 should work"
    );

    assert_eq!(
        root.readlink("abc").map_err(|err| err.kind()),
        Ok("b/c/file".into()),
        "__pathrs_inroot_symlink_v1 should produce a symlink to the target"
    );

    Ok(())
}

#[test]
fn rename_v1() -> Result<(), Error> {
    let root_dir = tests_common::create_basic_tree().context("create basic tree")?;
    let root = Root::open(root_dir.path()).context("Root::open basic tree")?;
    let path1 = capi_utils::path_to_cstring("b/c/file");
    let path2 = capi_utils::path_to_cstring("abc");

    assert_eq!(
        // SAFETY: Called with valid C-like arguments.
        capi_utils::call_capi(|| unsafe {
            capi::core::__pathrs_inroot_rename_v1(
                root.as_fd().into(),
                path1.as_ptr(),
                path2.as_ptr(),
                0,
            )
        })
        .map_err(|err| err.kind()),
        Ok(0),
        "__pathrs_inroot_symlink_v1 should work"
    );

    assert_matches!(
        root.resolve_nofollow("abc"),
        Ok(_),
        "__pathrs_inroot_rename_v1 should rename the target file (target appears)"
    );

    assert_matches!(
        root.resolve_nofollow("b/c/file").map_err(|err| err.kind()),
        Err(ErrorKind::OsError(Some(libc::ENOENT))),
        "__pathrs_inroot_rename_v1 should rename the target file (source disappears)"
    );

    Ok(())
}

#[test]
fn rename_v2() -> Result<(), Error> {
    let root_dir = tests_common::create_basic_tree().context("create basic tree")?;
    let root1 = Root::open(root_dir.path()).context("Root::open basic tree (#1)")?;
    let root2 = Root::open(root_dir.path()).context("Root::open basic tree (#2)")?;
    let path1 = capi_utils::path_to_cstring("b/c/file");
    let path2 = capi_utils::path_to_cstring("abc");

    assert_eq!(
        // SAFETY: Called with valid C-like arguments.
        capi_utils::call_capi(|| unsafe {
            capi::core::pathrs_inroot_rename(
                root1.as_fd().into(),
                path1.as_ptr(),
                root2.as_fd().into(),
                path2.as_ptr(),
                0,
            )
        })
        .map_err(|err| err.kind().errno()),
        Err(ErrorKind::InvalidArgument.errno()),
        "pathrs_inroot_rename@LIBPATHRS_0.2.5 should reject old_root_fd != new_root_fd"
    );

    Ok(())
}

#[test]
fn procfs_open_v1() -> Result<(), Error> {
    let path = capi_utils::path_to_cstring("stat");
    let oflags = OpenFlags::O_RDONLY | OpenFlags::O_NOFOLLOW;
    // SAFETY: Called with valid C-like arguments.
    let file = capi_utils::call_capi_fd(|| unsafe {
        capi::procfs::__pathrs_proc_open_v1(
            capi::procfs::CProcfsBase::PATHRS_PROC_THREAD_SELF,
            path.as_ptr(),
            oflags.bits() as _,
        )
    })
    .with_context(|| {
        format!("__pathrs_proc_open_v1(PATHRS_PROC_THREAD_SELF, {path:?}, {oflags:?})")
    })?;
    tests_common::check_oflags(&file, oflags)
        .with_context(|| format!("check procfs {path:?} {oflags:?} oflags"))?;

    Ok(())
}

#[test]
fn procfs_openat_v1() -> Result<(), Error> {
    let proc_rootfd = ProcfsHandle::new().context("ProcfsHandle::new")?;
    let path = capi_utils::path_to_cstring("stat");
    let oflags = OpenFlags::O_RDONLY | OpenFlags::O_NOFOLLOW;
    // SAFETY: Called with valid C-like arguments.
    let file = capi_utils::call_capi_fd(|| unsafe {
        capi::procfs::__pathrs_proc_openat_v1(
            proc_rootfd.as_fd().into(),
            capi::procfs::CProcfsBase::PATHRS_PROC_THREAD_SELF,
            path.as_ptr(),
            oflags.bits() as _,
        )
    })
    .with_context(|| {
        format!("__pathrs_proc_openat_v1(PATHRS_PROC_THREAD_SELF, {path:?}, {oflags:?})")
    })?;
    tests_common::check_oflags(&file, oflags)
        .with_context(|| format!("check procfs {path:?} {oflags:?} oflags"))?;

    Ok(())
}
