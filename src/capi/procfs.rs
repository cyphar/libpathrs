/*
 * libpathrs: safe path resolution on Linux
 * Copyright (C) 2019-2024 Aleksa Sarai <cyphar@cyphar.com>
 * Copyright (C) 2019-2024 SUSE LLC
 *
 * This program is free software: you can redistribute it and/or modify it under
 * the terms of the GNU Lesser General Public License as published by the Free
 * Software Foundation, either version 3 of the License, or (at your option) any
 * later version.
 *
 * This program is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A
 * PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU Lesser General Public License along
 * with this program. If not, see <https://www.gnu.org/licenses/>.
 */

use crate::{
    capi::{ret::IntoCReturn, utils},
    error::Error,
    flags::OpenFlags,
    procfs::{ProcfsBase, PROCFS_HANDLE},
};

use std::os::fd::RawFd;

use libc::{c_char, c_int, size_t};

/// Indicate what base directory should be used when doing operations with
/// pathrs_proc_*. This is necessary because /proc/thread-self is not present on
/// pre-3.17 kernels and so it may be necessary to emulate /proc/thread-self
/// access on those older kernels.
///
/// NOTE: Currently, operating on /proc/... directly is not supported.
///
/// [`ProcfsHandle`]: struct.ProcfsHandle.html
#[repr(C)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[allow(non_camel_case_types, dead_code)]
pub enum CProcfsBase {
    //PATHRS_PROC_ROOT, // TODO
    /// Use /proc/self. For most programs, this is the standard choice.
    PATHRS_PROC_SELF = 0x091D_5E1F,

    /// Use /proc/thread-self. In multi-threaded programs where one thread has a
    /// different CLONE_FS, it is possible for /proc/self to point the wrong
    /// thread and so /proc/thread-self may be necessary.
    ///
    /// NOTE: Using /proc/thread-self may require care if used from langauges
    /// where your code can change threads without warning and old threads can
    /// be killed (such as Go -- where you want to use runtime.LockOSThread).
    PATHRS_PROC_THREAD_SELF = 0x3EAD_5E1F,
}

impl From<CProcfsBase> for ProcfsBase {
    fn from(c_base: CProcfsBase) -> Self {
        match c_base {
            CProcfsBase::PATHRS_PROC_SELF => ProcfsBase::ProcSelf,
            CProcfsBase::PATHRS_PROC_THREAD_SELF => ProcfsBase::ProcThreadSelf,
        }
    }
}

/// Safely open a path inside a `/proc` handle.
///
/// Any bind-mounts or other over-mounts will (depending on what kernel features
/// are available) be detected and an error will be returned. Non-trailing
/// symlinks are followed but care is taken to ensure the symlinks are
/// legitimate.
///
/// Unless you intend to open a magic-link, `O_NOFOLLOW` should be set in flags.
/// Lookups with `O_NOFOLLOW` are guaranteed to never be tricked by bind-mounts
/// (on new enough Linux kernels).
///
/// If you wish to resolve a magic-link, you need to unset `O_NOFOLLOW`.
/// Unfortunately (if libpathrs is using the regular host `/proc` mount), this
/// lookup mode cannot protect you against an attacker that can modify the mount
/// table during this operation.
///
/// NOTE: Instead of using paths like `/proc/thread-self/fd`, `base` is used to
/// indicate what "base path" inside procfs is used. For example, to re-open a
/// file descriptor:
///
/// ```c
/// fd = pathrs_proc_open(PATHRS_PROC_THREAD_SELF, "fd/101", O_RDWR);
/// if (fd < 0) {
///     liberr = fd; // for use with pathrs_errorinfo()
///     goto err;
/// }
/// ```
///
/// # Return Value
///
/// On success, this function returns a file descriptor.
///
/// If an error occurs, this function will return a negative error code. To
/// retrieve information about the error (such as a string describing the error,
/// the system errno(7) value associated with the error, etc), use
/// pathrs_errorinfo().
#[no_mangle]
pub extern "C" fn pathrs_proc_open(base: CProcfsBase, path: *const c_char, flags: c_int) -> RawFd {
    || -> Result<_, Error> {
        let path = utils::parse_path(path)?;
        let oflags = OpenFlags::from_bits_retain(flags);

        match oflags.contains(OpenFlags::O_NOFOLLOW) {
            true => PROCFS_HANDLE.open(base.into(), path, oflags),
            false => PROCFS_HANDLE.open_follow(base.into(), path, oflags),
        }
    }()
    .into_c_return()
}

/// Safely read the contents of a symlink inside `/proc`.
///
/// As with `pathrs_proc_open`, any bind-mounts or other over-mounts will
/// (depending on what kernel features are available) be detected and an error
/// will be returned. Non-trailing symlinks are followed but care is taken to
/// ensure the symlinks are legitimate.
///
/// This function is effectively shorthand for
///
/// ```c
/// fd = pathrs_proc_open(base, path, O_PATH|O_NOFOLLOW);
/// if (fd < 0) {
///     liberr = fd; // for use with pathrs_errorinfo()
///     goto err;
/// }
/// copied = readlinkat(fd, "", linkbuf, linkbuf_size);
/// close(fd);
/// ```
///
/// # Return Value
///
/// On success, this function copies the symlink contents to `linkbuf` (up to
/// `linkbuf_size` bytes) and returns the full size of the symlink path buffer.
/// This function will not copy the trailing NUL byte, and the return size does
/// not include the NUL byte. A `NULL` `linkbuf` or invalid `linkbuf_size` are
/// treated as zero-size buffers.
///
/// NOTE: Unlike readlinkat(2), in the case where linkbuf is too small to
/// contain the symlink contents, pathrs_proc_readlink() will return *the number
/// of bytes it would have copied if the buffer was large enough*. This matches
/// the behaviour of pathrs_readlink().
///
/// If an error occurs, this function will return a negative error code. To
/// retrieve information about the error (such as a string describing the error,
/// the system errno(7) value associated with the error, etc), use
/// pathrs_errorinfo().
#[no_mangle]
pub extern "C" fn pathrs_proc_readlink(
    base: CProcfsBase,
    path: *const c_char,
    linkbuf: *mut c_char,
    linkbuf_size: size_t,
) -> c_int {
    || -> Result<_, Error> {
        let path = utils::parse_path(path)?;

        let link_target = PROCFS_HANDLE.readlink(base.into(), path)?;

        utils::copy_path_into_buffer(link_target, linkbuf, linkbuf_size)
    }()
    .into_c_return()
}
