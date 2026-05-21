// SPDX-License-Identifier: MPL-2.0 OR LGPL-3.0-or-later
/*
 * libpathrs: safe path resolution on Linux
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

use crate::capi::{ret::IntoCReturn, utils};

use std::{mem, ptr};

use bytemuck::{Pod, TransparentWrapper, Zeroable};
use libc::c_int;

#[derive(Debug, Clone, Copy, TransparentWrapper)]
#[repr(transparent)]
pub struct CStringPtr {
    ptr: *const u8,
}

// MSRV(1.88): Use #[derive(Default)].
impl Default for CStringPtr {
    fn default() -> Self {
        Self { ptr: ptr::null() }
    }
}

impl From<&'static str> for CStringPtr {
    fn from(s: &'static str) -> Self {
        s.as_ptr().into()
    }
}

impl From<*const u8> for CStringPtr {
    fn from(ptr: *const u8) -> Self {
        Self { ptr }
    }
}

// SAFETY: This pointer (which must be &'static) is serialised as a u64, which
// is safe to represent and parse. The ban on pointers is mostly about
// semantics, see <https://github.com/Lokathor/bytemuck/discussions/346>.
unsafe impl Zeroable for CStringPtr {}
unsafe impl Pod for CStringPtr {}

/// Represents the version information of the runtime libpathrs library.
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable)]
#[repr(C)]
pub struct VersionInfo {
    /// Pointer to static version string containing the crate version.
    /// cbindgen:rename-all="const char *"
    version_string: CStringPtr,
}

const VERSION_STRING: &str = concat!(env!("CARGO_PKG_VERSION"), "\0");

impl VersionInfo {
    pub(crate) fn new() -> Self {
        Self {
            version_string: VERSION_STRING.into(),
        }
    }
}

/// Return the version information of the libpathrs library at runtime.
///
/// This API is designed to be extensible. The caller must pass a pointer to a
/// contiguous buffer `dst` that is large enough to store `dst_size` bytes. This
/// buffer is then filled with version information. If the library has less data
/// than the given buffer size, the trailing bytes are zero-filled. If the
/// library has more data than the given buffer size the data is truncated and
/// (if the truncated data contained non-zero values), the return value
/// indicates what buffer size is needed to contain all version data.
///
/// # Return Value
///
/// On success, this function returns `0` or (if the version data was truncated)
/// a positive integer that is the size of the buffer needed to store all data.
///
/// If an error occurs, this function will return a negative error code. To
/// retrieve information about the error (such as a string describing the error,
/// the system errno(7) value associated with the error, etc), use
/// pathrs_errorinfo().
#[no_mangle]
pub unsafe extern "C" fn pathrs_version(dst: *mut VersionInfo, dst_size: usize) -> c_int {
    {
        let v = VersionInfo::new();
        // SAFETY: C caller guarantees buffer is at least dst_size and can be
        // written to.
        unsafe { utils::copy_into_extensible_struct(dst, dst_size, &v) }.map(|truncated| {
            if truncated {
                mem::size_of::<VersionInfo>() as isize
            } else {
                0isize // not truncated, return 0
            }
        })
    }
    .into_c_return()
}
utils::symver! {
    fn pathrs_version <- (pathrs_version, version = "LIBPATHRS_0.2.5", default);
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{mem, ptr};

    use bytemuck::TransparentWrapper;
    use pretty_assertions::assert_eq;

    #[test]
    fn version() {
        let mut info = VersionInfo::default();

        let ret = unsafe { pathrs_version(&mut info as *mut _, mem::size_of::<VersionInfo>()) };
        assert_eq!(
            ret, 0,
            "pathrs_version with sizeof(VersionInfo) should not indicate truncation",
        );
        assert_eq!(
            TransparentWrapper::peel_ref(&info.version_string),
            &VERSION_STRING.as_ptr(),
            "pathrs_version should set info.version_string to VERSION_STRING pointer",
        );
        // TODO: Parse CString and compare to CARGO_PKG_VERSION?
    }

    #[test]
    fn version_zerosize() {
        let mut info = VersionInfo::default();

        let ret = unsafe { pathrs_version(&mut info as *mut _, 0) };
        assert_eq!(
            ret,
            mem::size_of::<VersionInfo>() as i32,
            "pathrs_version with zero size should indicate truncation",
        );
        assert_eq!(
            TransparentWrapper::peel_ref(&info.version_string),
            &ptr::null(),
            "pathrs_version should not modify info.version_string with zero size",
        );
    }

    #[test]
    fn version_null() {
        let ret = unsafe { pathrs_version(ptr::null_mut(), 0) };
        assert_eq!(
            ret,
            mem::size_of::<VersionInfo>() as i32,
            "pathrs_version with NULL and zero size should indicate truncation",
        );
    }
}
