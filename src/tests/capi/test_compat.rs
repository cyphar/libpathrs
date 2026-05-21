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
    tests::{capi::utils as capi_utils, common as tests_common, traits::ErrorImpl},
    utils::FdExt,
    Root,
};

use std::os::unix::{fs::MetadataExt, io::AsFd};

use anyhow::{Context, Error};
use pretty_assertions::{assert_eq, assert_matches};

#[test]
fn hardlink_v1() -> Result<(), Error> {
    let root_dir = tests_common::create_basic_tree()?;
    let root = Root::open(root_dir.path())?;

    assert_eq!(
        // SAFETY: Called with valid C-like arguments.
        capi_utils::call_capi(|| unsafe {
            capi::core::__pathrs_inroot_hardlink_v1(
                root.as_fd().into(),
                capi_utils::path_to_cstring("abc").into_raw(),
                capi_utils::path_to_cstring("b/c/file").into_raw(),
            )
        })
        .map_err(|err| err.kind()),
        Ok(0),
        "__pathrs_inroot_hardlink_v1 should work"
    );

    let old_meta = root
        .resolve_nofollow("b/c/file")?
        .metadata()
        .context("fstat b/c/file")?;
    let new_meta = root
        .resolve_nofollow("abc")
        .context("hardlink abc should've been created")?
        .metadata()
        .context("fstat hardlink abc")?;
    assert_eq!(
        old_meta.ino(),
        new_meta.ino(),
        "inode numbers of hardlinks with __pathrs_inroot_hardlink_v1 should be the same",
    );

    Ok(())
}

#[test]
fn hardlink_v2() -> Result<(), Error> {
    let root_dir = tests_common::create_basic_tree()?;
    let root1 = Root::open(root_dir.path())?;
    let root2 = Root::open(root_dir.path())?;

    assert_eq!(
        // SAFETY: Called with valid C-like arguments.
        capi_utils::call_capi(|| unsafe {
            capi::core::pathrs_inroot_hardlink(
                root1.as_fd().into(),
                capi_utils::path_to_cstring("abc").into_raw(),
                root2.as_fd().into(),
                capi_utils::path_to_cstring("b/c/file").into_raw(),
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
                capi_utils::path_to_cstring("abc").into_raw(),
                root1.as_fd().into(),
                capi_utils::path_to_cstring("b/c/file").into_raw(),
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
    let root_dir = tests_common::create_basic_tree()?;
    let root = Root::open(root_dir.path())?;

    assert_eq!(
        // SAFETY: Called with valid C-like arguments.
        capi_utils::call_capi(|| unsafe {
            capi::core::__pathrs_inroot_symlink_v1(
                root.as_fd().into(),
                capi_utils::path_to_cstring("abc").into_raw(),
                capi_utils::path_to_cstring("b/c/file").into_raw(),
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
    let root_dir = tests_common::create_basic_tree()?;
    let root = Root::open(root_dir.path())?;

    assert_eq!(
        // SAFETY: Called with valid C-like arguments.
        capi_utils::call_capi(|| unsafe {
            capi::core::__pathrs_inroot_rename_v1(
                root.as_fd().into(),
                capi_utils::path_to_cstring("b/c/file").into_raw(),
                capi_utils::path_to_cstring("abc").into_raw(),
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
    let root_dir = tests_common::create_basic_tree()?;
    let root1 = Root::open(root_dir.path())?;
    let root2 = Root::open(root_dir.path())?;

    assert_eq!(
        // SAFETY: Called with valid C-like arguments.
        capi_utils::call_capi(|| unsafe {
            capi::core::pathrs_inroot_rename(
                root1.as_fd().into(),
                capi_utils::path_to_cstring("abc").into_raw(),
                root2.as_fd().into(),
                capi_utils::path_to_cstring("b/c/file").into_raw(),
                0,
            )
        })
        .map_err(|err| err.kind().errno()),
        Err(ErrorKind::InvalidArgument.errno()),
        "pathrs_inroot_rename@LIBPATHRS_0.2.5 should reject old_root_fd != new_root_fd"
    );

    Ok(())
}
