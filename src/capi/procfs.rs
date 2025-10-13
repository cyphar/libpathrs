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

use crate::{
    capi::{
        ret::IntoCReturn,
        utils::{self, CBorrowedFd},
    },
    error::{Error, ErrorExt, ErrorImpl},
    flags::OpenFlags,
    procfs::{ProcfsBase, ProcfsHandle, ProcfsHandleBuilder, ProcfsHandleRef},
};

use std::os::unix::io::{AsRawFd, IntoRawFd, OwnedFd, RawFd};

use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use libc::{c_char, c_int, size_t};
use open_enum::open_enum;

/// Bits in `pathrs_proc_base_t` that indicate the type of the base value.
///
/// NOTE: This is used internally by libpathrs. You should avoid using this
/// macro if possible.
pub const __PATHRS_PROC_TYPE_MASK: u64 = 0xFFFF_FFFF_0000_0000;

/// Bits in `pathrs_proc_base_t` that must be set for "special" `PATHRS_PROC_*`
/// values.
const __PATHRS_PROC_TYPE_SPECIAL: u64 = 0xFFFF_FFFE_0000_0000;

// Make sure that __PATHRS_PROC_TYPE_SPECIAL only uses bits in the mask.
static_assertions::const_assert_eq!(
    __PATHRS_PROC_TYPE_SPECIAL,
    __PATHRS_PROC_TYPE_SPECIAL & __PATHRS_PROC_TYPE_MASK,
);

/// Bits in `pathrs_proc_base_t` that must be set for `/proc/$pid` values. Don't
/// use this directly, instead use `PATHRS_PROC_PID(n)` to convert a PID to an
/// appropriate `pathrs_proc_base_t` value.
///
/// NOTE: This is used internally by libpathrs. You should avoid using this
/// macro if possible.
// For future-proofing the top 32 bits are blocked off by the mask, but in case
// we ever need to expand the size of pid_t (incredibly unlikely) we only use
// the top-most bit.
pub const __PATHRS_PROC_TYPE_PID: u64 = 0x8000_0000_0000_0000;

// Make sure that __PATHRS_PROC_TYPE_PID only uses bits in the mask.
static_assertions::const_assert_eq!(
    __PATHRS_PROC_TYPE_PID,
    __PATHRS_PROC_TYPE_PID & __PATHRS_PROC_TYPE_MASK,
);

/// Indicate what base directory should be used when doing operations with
/// `pathrs_proc_*`. In addition to the values defined here, the following
/// macros can be used for other values:
///
///  * `PATHRS_PROC_PID(pid)` refers to the `/proc/<pid>` directory for the
///    process with PID (or TID) `pid`.
///
///    Note that this operation is inherently racy and should probably avoided
///    for most uses -- see the block comment above `PATHRS_PROC_PID(n)` for
///    more details.
///
/// Unknown values will result in an error being returned.
// NOTE: We need to open-code the values in the definition because cbindgen
// cannot yet evaluate constexprs (see <>) and both Go's CGo and Python's cffi
// struggle to deal with non-constant values (actually CGo struggles even with
// unsigned literals -- see <https://github.com/golang/go/issues/39136>).
#[open_enum]
#[repr(u64)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[allow(non_camel_case_types, dead_code)]
#[allow(clippy::unusual_byte_groupings)] // FIXME: workaround for <https://github.com/rust-lang/rust-clippy/issues/15210>
pub enum CProcfsBase {
    /// Use /proc. Note that this mode may be more expensive because we have
    /// to take steps to try to avoid leaking unmasked procfs handles, so you
    /// should use PATHRS_PROC_SELF if you can.
    PATHRS_PROC_ROOT = 0xFFFF_FFFE_7072_6F63u64, // "proc"

    /// Use /proc/self. For most programs, this is the standard choice.
    PATHRS_PROC_SELF = 0xFFFF_FFFE_091D_5E1Fu64, // pid-self

    /// Use /proc/thread-self. In multi-threaded programs where one thread has a
    /// different CLONE_FS, it is possible for /proc/self to point the wrong
    /// thread and so /proc/thread-self may be necessary.
    ///
    /// NOTE: Using /proc/thread-self may require care if used from languages
    /// where your code can change threads without warning and old threads can
    /// be killed (such as Go -- where you want to use runtime.LockOSThread).
    PATHRS_PROC_THREAD_SELF = 0xFFFF_FFFE_3EAD_5E1Fu64, // thread-self
}

// Make sure the defined special values have the right type flag and the right
// values. The value checks are critical because we must not change these --
// changing them will break API compatibility silently.

static_assertions::const_assert_eq!(0xFFFFFFFE70726F63, CProcfsBase::PATHRS_PROC_ROOT.0);
static_assertions::const_assert_eq!(
    __PATHRS_PROC_TYPE_SPECIAL | 0x7072_6F63,
    CProcfsBase::PATHRS_PROC_ROOT.0,
);
static_assertions::const_assert_eq!(
    __PATHRS_PROC_TYPE_SPECIAL,
    CProcfsBase::PATHRS_PROC_ROOT.0 & __PATHRS_PROC_TYPE_MASK,
);

static_assertions::const_assert_eq!(0xFFFFFFFE091D5E1F, CProcfsBase::PATHRS_PROC_SELF.0);
static_assertions::const_assert_eq!(
    __PATHRS_PROC_TYPE_SPECIAL | 0x091D_5E1F,
    CProcfsBase::PATHRS_PROC_SELF.0,
);
static_assertions::const_assert_eq!(
    __PATHRS_PROC_TYPE_SPECIAL,
    CProcfsBase::PATHRS_PROC_SELF.0 & __PATHRS_PROC_TYPE_MASK,
);

static_assertions::const_assert_eq!(0xFFFFFFFE3EAD5E1F, CProcfsBase::PATHRS_PROC_THREAD_SELF.0);
static_assertions::const_assert_eq!(
    __PATHRS_PROC_TYPE_SPECIAL | 0x3EAD_5E1F,
    CProcfsBase::PATHRS_PROC_THREAD_SELF.0,
);
static_assertions::const_assert_eq!(
    __PATHRS_PROC_TYPE_SPECIAL,
    CProcfsBase::PATHRS_PROC_THREAD_SELF.0 & __PATHRS_PROC_TYPE_MASK,
);

impl TryFrom<CProcfsBase> for ProcfsBase {
    type Error = Error;

    fn try_from(c_base: CProcfsBase) -> Result<Self, Self::Error> {
        // Cannot be used inline in a pattern.
        const U32_MAX: u64 = u32::MAX as _;
        const U32_MAX_PLUS_ONE: u64 = U32_MAX + 1;

        match c_base {
            CProcfsBase::PATHRS_PROC_ROOT => Ok(ProcfsBase::ProcRoot),
            CProcfsBase::PATHRS_PROC_SELF => Ok(ProcfsBase::ProcSelf),
            CProcfsBase::PATHRS_PROC_THREAD_SELF => Ok(ProcfsBase::ProcThreadSelf),
            CProcfsBase(arg) => match (
                arg & __PATHRS_PROC_TYPE_MASK,
                arg & !__PATHRS_PROC_TYPE_MASK,
            ) {
                // Make sure that we never run into a situation where the
                // argument value doesn't fit in a u32. If we ever need to
                // support this, we will need additional code changes.
                (_, value @ U32_MAX_PLUS_ONE..) => {
                    // This should really never actually happen, so ensure we
                    // have a compile-time check to avoid it.
                    static_assertions::const_assert_eq!(!__PATHRS_PROC_TYPE_MASK, u32::MAX as _);
                    // And mark this branch as unreachable.
                    unreachable!("the value portion of CProcfsBase({arg:#x}) cannot be larger than u32 (but {value:#x} is)");
                }

                // PATHRS_PROC_PID(pid)
                (__PATHRS_PROC_TYPE_PID, pid @ 1..=U32_MAX) => {
                    // Just make sure...
                    static_assertions::const_assert_eq!(U32_MAX, u32::MAX as u64);
                    // We can be sure it's okay to cast to u32, as we've checked
                    // statically and at runtime that the value is within the
                    // correct range to not truncate bits.
                    Ok(ProcfsBase::ProcPid(pid as u32))
                }

                // Error fallbacks for invalid subvalues or types.
                (base_type, value) => Err(ErrorImpl::InvalidArgument {
                    name: "procfs base".into(),
                    description: match base_type {
                        __PATHRS_PROC_TYPE_SPECIAL => format!("{arg:#X} is an invalid special procfs base (unknown sub-value {value:#X})"),
                        __PATHRS_PROC_TYPE_PID => format!("pid {value} is an invalid value for PATHRS_PROC_PID"),
                        _ => format!("{arg:#X} has an unknown procfs base type {base_type:#X}"),
                    }.into(),
                }.into()),
            }
            .wrap("the procfs base must be one of the PATHRS_PROC_* values or PATHRS_PROC_PID(n)")
        }
    }
}

#[cfg(test)]
impl From<ProcfsBase> for CProcfsBase {
    fn from(base: ProcfsBase) -> Self {
        match base {
            ProcfsBase::ProcPid(pid) => {
                // TODO: See if we can add some kind of static assertion that
                //       the type of the pid is not larger than the reserved
                //       block in __PATHRS_PROC_TYPE_MASK. Unfortunately Rust
                //       doesn't have a way of doing typeof(pid)... Maybe
                //       pattern_types would let us do this with something like
                //       ProcfsBase::ProcPid::0::MAX?
                //
                // static_assertions::const_assert_eq!(
                //   type_of<pid>::MAX & _PATHRS_PROC_TYPE_MASK,
                //   0,
                // );
                // static_assertions::const_assert_eq!(type_of<pid>::MAX, u32::MAX);

                // We know this to be true from the check in the above TryFrom
                // impl for ProcfsBase, but add an assertion here since we
                // cannot actually verify this statically at the moment.

                #[allow(clippy::absurd_extreme_comparisons)]
                {
                    assert!(pid <= u32::MAX, "pid in CProcfsBase must fit inside a u32");
                }
                assert_eq!(
                    pid as u64 & __PATHRS_PROC_TYPE_MASK, 0,
                    "invalid pid found when converting to CProcfsBase -- pid {pid} includes type bits ({__PATHRS_PROC_TYPE_MASK:#X})"
                );
                CProcfsBase(__PATHRS_PROC_TYPE_PID | pid as u64)
            }
            ProcfsBase::ProcRoot => CProcfsBase::PATHRS_PROC_ROOT,
            ProcfsBase::ProcSelf => CProcfsBase::PATHRS_PROC_SELF,
            ProcfsBase::ProcThreadSelf => CProcfsBase::PATHRS_PROC_THREAD_SELF,
        }
    }
}

/// A sentinel value to tell `pathrs_proc_*` methods to use the default procfs
/// root handle (which may be globally cached).
pub const PATHRS_PROC_DEFAULT_ROOTFD: CBorrowedFd<'static> =
    // SAFETY: The lifetime of fake fd values are 'static.
    unsafe { CBorrowedFd::from_raw_fd(-libc::EBADF) };

fn parse_proc_rootfd<'fd>(fd: CBorrowedFd<'fd>) -> Result<ProcfsHandleRef<'fd>, Error> {
    match fd {
        PATHRS_PROC_DEFAULT_ROOTFD => ProcfsHandle::new(),
        _ => ProcfsHandleRef::try_from_borrowed_fd(fd.try_as_borrowed_fd()?),
    }
}

// This is needed because the macro expansion cbindgen produces for
// bitflags! is not really usable for regular structs.
/// Construct a completely unmasked procfs handle.
///
/// This is equivalent to [`ProcfsHandleBuilder::unmasked`], and is meant as
/// a flag argument to [`ProcfsOpenFlags`] (the `flags` field in `struct
/// pathrs_procfs_open_how`) for use with pathrs_procfs_open().
pub const PATHRS_PROCFS_NEW_UNMASKED: u64 = 0x0000_0000_0000_0001;

bitflags! {
    #[repr(C)]
    #[derive(Default, Debug, Clone, Copy, Pod, Zeroable)]
    pub struct ProcfsOpenFlags: u64 {
        const PATHRS_PROCFS_NEW_UNMASKED = PATHRS_PROCFS_NEW_UNMASKED;
        // NOTE: Make sure to add a `pub const` for any new flags to make
        // sure they show up when cbindgen generates our header.
    }
}

impl ProcfsOpenFlags {
    const fn contains_unknown_bits(&self) -> bool {
        Self::from_bits(self.bits()).is_none()
    }
}

static_assertions::const_assert_eq!(
    ProcfsOpenFlags::PATHRS_PROCFS_NEW_UNMASKED.contains_unknown_bits(),
    false,
);
static_assertions::const_assert_eq!(
    ProcfsOpenFlags::from_bits_retain(0x1000_0000).contains_unknown_bits(),
    true,
);
static_assertions::const_assert_eq!(
    ProcfsOpenFlags::from_bits_retain(0xF000_0001).contains_unknown_bits(),
    true,
);

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod, Zeroable)]
pub struct ProcfsOpenHow {
    pub flags: ProcfsOpenFlags,
}

impl ProcfsOpenHow {
    fn into_builder(self) -> Result<ProcfsHandleBuilder, Error> {
        let mut builder = ProcfsHandleBuilder::new();

        if self.flags.contains_unknown_bits() {
            return Err(ErrorImpl::InvalidArgument {
                name: "flags".into(),
                description: format!(
                    "contains unknown flag bits {:#x}",
                    self.flags.difference(ProcfsOpenFlags::all())
                )
                .into(),
            })?;
        }
        if self
            .flags
            .contains(ProcfsOpenFlags::PATHRS_PROCFS_NEW_UNMASKED)
        {
            builder.set_unmasked();
        }

        Ok(builder)
    }
}

/// Create a new (custom) procfs root handle.
///
/// This is effectively a C wrapper around [`ProcfsHandleBuilder`], allowing you
/// to create a custom procfs root handle that can be used with other
/// `pathrs_proc_*at` methods.
///
/// While most users should just use `PATHRS_PROC_DEFAULT_ROOTFD` (or the
/// non-`at` variants of `pathrs_proc_*`), creating an unmasked procfs root
/// handle (using `PATHRS_PROCFS_NEW_UNMASKED`) can be useful for programs that
/// need to operate on a lot of global procfs files. (Note that accessing global
/// procfs files does not *require* creating a custom procfs handle --
/// `pathrs_proc_*` will automatically create a global-friendly handle
/// internally when necessary but will close it immediately after operating on
/// it.)
///
/// # Extensible Structs
///
/// The [`ProcfsOpenHow`] (`struct pathrs_procfs_open_how`) argument is
/// designed to be extensible, modelled after the extensible structs scheme used
/// by Linux (for syscalls such as [clone3(2)], [openat2(2)] and other such
/// syscalls). Normally one would use symbol versioning to achieve this, but
/// unfortunately Rust's symbol versioning support is incredibly primitive (one
/// might even say "non-existent") and so this system is more robust, even if
/// the calling convention is a little strange for userspace libraries.
///
/// In addition to a pointer argument, the caller must also provide the size of
/// the structure it is passing. By providing this information, it is possible
/// for `pathrs_procfs_open()` to provide both forwards- and
/// backwards-compatibility, with size acting as an implicit version number.
/// (Because new extension fields will always be appended, the structure size
/// will always increase.)
///
/// If we let `usize` be the structure specified by the caller, and `lsize` be
/// the size of the structure internal to libpathrs, then there are three cases
/// to consider:
///
/// * If `usize == lsize`, then there is no version mismatch and the structure
///   provided by the caller can be used verbatim.
/// * If `usize < lsize`, then there are some extension fields which libpathrs
///   supports that the caller does not. Because a zero value in any added
///   extension field signifies a no-op, libpathrs treats all of the extension
///   fields not provided by the caller as having zero values. This provides
///   backwards-compatibility.
/// * If `usize > lsize`, then there are some extension fields which the caller
///   is aware of but this version of libpathrs does not support. Because any
///   extension field must have its zero values signify a no-op, libpathrs can
///   safely ignore the unsupported extension fields if they are all-zero. If
///   any unsupported extension fields are nonzero, then an `E2BIG` error is
///   returned. This provides forwards-compatibility.
///
/// Because the definition of `struct pathrs_procfs_open_how` may open in the
/// future
///
/// Because the definition of `struct pathrs_procfs_open_how` may change in the
/// future (with new fields being added when headers are updated), callers
/// should zero-fill the structure to ensure that recompiling the program with
/// new headers will not result in spurious errors at run time. The simplest
/// way is to use a designated initialiser:
///
/// ```c
///     struct pathrs_procfs_open_how how = {
///         .flags = PATHRS_PROCFS_NEW_UNMASKED,
///     };
/// ```
///
/// or explicitly using `memset(3)` or similar:
///
/// ```c
/// struct pathrs_procfs_open_how how;
/// memset(&how, 0, sizeof(how));
/// how.flags = PATHRS_PROCFS_NEW_UNMASKED;
/// ```
///
/// # Return Value
///
/// On success, this function returns *either* a file descriptor *or*
/// `PATHRS_PROC_DEFAULT_ROOTFD` (this is a negative number, equal to `-EBADF`).
/// The file descriptor will have the `O_CLOEXEC` flag automatically applied.
///
/// If an error occurs, this function will return a negative error code. To
/// retrieve information about the error (such as a string describing the error,
/// the system errno(7) value associated with the error, etc), use
/// pathrs_errorinfo().
///
/// [clone3(2)]: https://www.man7.org/linux/man-pages/man2/clone3.2.html
/// [openat2(2)]: https://www.man7.org/linux/man-pages/man2/openat2.2.html
#[no_mangle]
pub unsafe extern "C" fn pathrs_procfs_open(args: *const ProcfsOpenHow, size: usize) -> RawFd {
    || -> Result<_, Error> {
        unsafe { utils::copy_from_extensible_struct(args, size) }?
            .into_builder()?
            .build()
            .map(ProcfsHandle::into_owned_fd)
    }()
    .map(|fd| match fd {
        Some(fd) => fd.into_raw_fd(),
        None => PATHRS_PROC_DEFAULT_ROOTFD.as_raw_fd(),
    })
    .into_c_return()
}

/// `pathrs_proc_open` but with a caller-provided file descriptor for `/proc`.
///
/// Internally, `pathrs_proc_open` will attempt to use a cached copy of a very
/// restricted `/proc` handle (a detached mount object with `subset=pid` and
/// `hidepid=4`). If a user requests a global `/proc` file, a temporary handle
/// capable of accessing global files is created and destroyed after the
/// operation completes.
///
/// For most users, this is more than sufficient. However, if a user needs to
/// operate on many global `/proc` files, the cost of creating handles can get
/// quite expensive. `pathrs_proc_openat` allows a user to manually manage the
/// global-friendly `/proc` handle. Note that passing a `subset=pid` file
/// descriptor to `pathrs_proc_openat` will *not* stop the automatic creation of
/// a global-friendly handle internally if necessary.
///
/// In order to get the behaviour of `pathrs_proc_open`, you can pass the
/// special value `PATHRS_PROC_DEFAULT_ROOTFD` (`-EBADF`) as the `proc_rootfd`
/// argument.
///
/// # Return Value
///
/// On success, this function returns a file descriptor. The file descriptor
/// will have the `O_CLOEXEC` flag automatically applied.
///
/// If an error occurs, this function will return a negative error code. To
/// retrieve information about the error (such as a string describing the error,
/// the system errno(7) value associated with the error, etc), use
/// pathrs_errorinfo().
#[no_mangle]
pub unsafe extern "C" fn pathrs_proc_openat(
    proc_rootfd: CBorrowedFd<'_>,
    base: CProcfsBase,
    path: *const c_char,
    flags: c_int,
) -> RawFd {
    || -> Result<_, Error> {
        let base = base.try_into()?;
        let path = unsafe { utils::parse_path(path) }?; // SAFETY: C caller guarantees path is safe.
        let oflags = OpenFlags::from_bits_retain(flags);
        let procfs = parse_proc_rootfd(proc_rootfd)?;

        match oflags.contains(OpenFlags::O_NOFOLLOW) {
            true => procfs.open(base, path, oflags),
            false => procfs.open_follow(base, path, oflags),
        }
    }()
    .map(OwnedFd::from)
    .into_c_return()
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
/// if (IS_PATHRS_ERR(fd)) {
///     liberr = fd; // for use with pathrs_errorinfo()
///     goto err;
/// }
/// ```
///
/// # Return Value
///
/// On success, this function returns a file descriptor. The file descriptor
/// will have the `O_CLOEXEC` flag automatically applied.
///
/// If an error occurs, this function will return a negative error code. To
/// retrieve information about the error (such as a string describing the error,
/// the system errno(7) value associated with the error, etc), use
/// pathrs_errorinfo().
#[no_mangle]
pub unsafe extern "C" fn pathrs_proc_open(
    base: CProcfsBase,
    path: *const c_char,
    flags: c_int,
) -> RawFd {
    pathrs_proc_openat(PATHRS_PROC_DEFAULT_ROOTFD, base, path, flags)
}

/// `pathrs_proc_readlink` but with a caller-provided file descriptor for
/// `/proc`.
///
/// See the documentation of pathrs_proc_openat() for when this API might be
/// useful.
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
/// the behaviour of pathrs_inroot_readlink().
///
/// If an error occurs, this function will return a negative error code. To
/// retrieve information about the error (such as a string describing the error,
/// the system errno(7) value associated with the error, etc), use
/// pathrs_errorinfo().
#[no_mangle]
pub unsafe extern "C" fn pathrs_proc_readlinkat(
    proc_rootfd: CBorrowedFd<'_>,
    base: CProcfsBase,
    path: *const c_char,
    linkbuf: *mut c_char,
    linkbuf_size: size_t,
) -> c_int {
    || -> Result<_, Error> {
        let base = base.try_into()?;
        let path = unsafe { utils::parse_path(path) }?; // SAFETY: C caller guarantees path is safe.
        let procfs = parse_proc_rootfd(proc_rootfd)?;
        let link_target = procfs.readlink(base, path)?;
        // SAFETY: C caller guarantees buffer is at least linkbuf_size and can
        // be written to.
        unsafe { utils::copy_path_into_buffer(link_target, linkbuf, linkbuf_size) }
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
/// if (IS_PATHRS_ERR(fd)) {
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
/// the behaviour of pathrs_inroot_readlink().
///
/// If an error occurs, this function will return a negative error code. To
/// retrieve information about the error (such as a string describing the error,
/// the system errno(7) value associated with the error, etc), use
/// pathrs_errorinfo().
#[no_mangle]
pub unsafe extern "C" fn pathrs_proc_readlink(
    base: CProcfsBase,
    path: *const c_char,
    linkbuf: *mut c_char,
    linkbuf_size: size_t,
) -> c_int {
    pathrs_proc_readlinkat(
        PATHRS_PROC_DEFAULT_ROOTFD,
        base,
        path,
        linkbuf,
        linkbuf_size,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        capi::{error as capi_error, utils::Leakable},
        error::ErrorKind,
        procfs::ProcfsBase,
    };

    use std::{
        mem,
        os::unix::io::{FromRawFd, OwnedFd},
    };

    use pretty_assertions::assert_eq;

    #[test]
    fn procfsbase_try_from_crepr_procroot() {
        assert_eq!(
            ProcfsBase::try_from(CProcfsBase::PATHRS_PROC_ROOT).map_err(|e| e.kind()),
            Ok(ProcfsBase::ProcRoot),
            "PATHRS_PROC_ROOT.try_into()"
        );
    }

    #[test]
    fn procfsbase_try_from_crepr_procself() {
        assert_eq!(
            ProcfsBase::try_from(CProcfsBase::PATHRS_PROC_SELF).map_err(|e| e.kind()),
            Ok(ProcfsBase::ProcSelf),
            "PATHRS_PROC_SELF.try_into()"
        );
    }

    #[test]
    fn procfsbase_try_from_crepr_procthreadself() {
        assert_eq!(
            ProcfsBase::try_from(CProcfsBase::PATHRS_PROC_THREAD_SELF).map_err(|e| e.kind()),
            Ok(ProcfsBase::ProcThreadSelf),
            "PATHRS_PROC_THREAD_SELF.try_into()"
        );
    }

    #[test]
    fn procfsbase_try_from_crepr_procpid() {
        assert_eq!(
            ProcfsBase::try_from(CProcfsBase(__PATHRS_PROC_TYPE_PID | 1)).map_err(|e| e.kind()),
            Ok(ProcfsBase::ProcPid(1)),
            "PATHRS_PROC_PID(12345).try_into()"
        );
        assert_eq!(
            ProcfsBase::try_from(CProcfsBase(__PATHRS_PROC_TYPE_PID | 12345)).map_err(|e| e.kind()),
            Ok(ProcfsBase::ProcPid(12345)),
            "PATHRS_PROC_PID(12345).try_into()"
        );
        assert_eq!(
            ProcfsBase::try_from(CProcfsBase(__PATHRS_PROC_TYPE_PID | u32::MAX as u64))
                .map_err(|e| e.kind()),
            Ok(ProcfsBase::ProcPid(u32::MAX)),
            "PATHRS_PROC_PID(u32::MAX).try_into()"
        );
    }

    #[test]
    fn procfsbase_try_from_crepr_procspecial_invalid() {
        // Invalid __PATHRS_PROC_TYPE_SPECIAL values.
        assert_eq!(
            ProcfsBase::try_from(CProcfsBase(__PATHRS_PROC_TYPE_SPECIAL)).map_err(|e| e.kind()),
            Err(ErrorKind::InvalidArgument),
            "__PATHRS_PROC_TYPE_SPECIAL.try_into() -- invalid type"
        );
        assert_eq!(
            ProcfsBase::try_from(CProcfsBase(__PATHRS_PROC_TYPE_SPECIAL | 0xDEADBEEF))
                .map_err(|e| e.kind()),
            Err(ErrorKind::InvalidArgument),
            "(__PATHRS_PROC_TYPE_SPECIAL | 0xDEADBEEF).try_into() -- invalid type"
        );
    }

    #[test]
    fn procfsbase_try_from_crepr_procpid_invalid() {
        // 0 is an invalid pid.
        assert_eq!(
            ProcfsBase::try_from(CProcfsBase(__PATHRS_PROC_TYPE_PID)).map_err(|e| e.kind()),
            Err(ErrorKind::InvalidArgument),
            "PATHRS_PROC_PID(0).try_into() -- invalid pid"
        );
        // u32::MAX + 1 is an invalid value for multiple reasons.
        assert_eq!(
            ProcfsBase::try_from(CProcfsBase(__PATHRS_PROC_TYPE_PID | (u32::MAX as u64 + 1)))
                .map_err(|e| e.kind()),
            Err(ErrorKind::InvalidArgument),
            "PATHRS_PROC_PID(u32::MAX + 1).try_into() -- invalid pid"
        );
    }

    #[test]
    fn procfsbase_try_from_crepr_proctype_invalid() {
        // Invalid __PATHRS_PROC_TYPE_MASK values.
        assert_eq!(
            ProcfsBase::try_from(CProcfsBase(0xDEAD_BEEF_0000_0001)).map_err(|e| e.kind()),
            Err(ErrorKind::InvalidArgument),
            "0xDEAD_BEEF_0000_0001.try_into() -- invalid type"
        );
        assert_eq!(
            ProcfsBase::try_from(CProcfsBase(0xDEAD_BEEF_3EAD_5E1F)).map_err(|e| e.kind()),
            Err(ErrorKind::InvalidArgument),
            "0xDEAD_BEEF_3EAD_5E1F.try_into() -- invalid type"
        );
        assert_eq!(
            ProcfsBase::try_from(CProcfsBase(__PATHRS_PROC_TYPE_MASK)).map_err(|e| e.kind()),
            Err(ErrorKind::InvalidArgument),
            "__PATHRS_PROC_TYPE_MASK.try_into() -- invalid type"
        );
    }

    #[test]
    fn procfsbase_try_from_crepr_invalid() {
        // Plain values are invalid.
        assert_eq!(
            ProcfsBase::try_from(CProcfsBase(0)).map_err(|e| e.kind()),
            Err(ErrorKind::InvalidArgument),
            "(0).try_into() -- invalid value"
        );
        assert_eq!(
            ProcfsBase::try_from(CProcfsBase(0xDEADBEEF)).map_err(|e| e.kind()),
            Err(ErrorKind::InvalidArgument),
            "(0xDEADBEEF).try_into() -- invalid value"
        );
    }

    #[test]
    fn procfsbase_into_crepr_procroot() {
        assert_eq!(
            CProcfsBase::from(ProcfsBase::ProcRoot),
            CProcfsBase::PATHRS_PROC_ROOT,
            "ProcRoot.into() == PATHRS_PROC_ROOT"
        );
    }

    #[test]
    fn procfsbase_into_crepr_procself() {
        assert_eq!(
            CProcfsBase::from(ProcfsBase::ProcSelf),
            CProcfsBase::PATHRS_PROC_SELF,
            "ProcSelf.into() == PATHRS_PROC_SELF"
        );
    }

    #[test]
    fn procfsbase_into_crepr_procthreadself() {
        assert_eq!(
            CProcfsBase::from(ProcfsBase::ProcThreadSelf),
            CProcfsBase::PATHRS_PROC_THREAD_SELF,
            "ProcThreadSelf.into() == PATHRS_PROC_THREAD_SELF"
        );
    }

    #[test]
    fn procfsbase_into_crepr_procpid() {
        assert_eq!(
            CProcfsBase::from(ProcfsBase::ProcPid(1)),
            CProcfsBase(__PATHRS_PROC_TYPE_PID | 1),
            "ProcPid(1).into() == 1"
        );
        assert_eq!(
            CProcfsBase::from(ProcfsBase::ProcPid(1122334455)),
            CProcfsBase(__PATHRS_PROC_TYPE_PID | 1122334455),
            "ProcPid(1122334455).into() == 1122334455"
        );
    }

    fn check_round_trip(rust: ProcfsBase, c: CProcfsBase) {
        let c_to_rust: ProcfsBase = c.try_into().expect("should be valid value");
        assert_eq!(
            rust, c_to_rust,
            "c-to-rust ProcfsBase conversion ({c:?}.try_into())"
        );

        let rust_to_c: CProcfsBase = rust.into();
        assert_eq!(
            c, rust_to_c,
            "rust-to-c ProcfsBase conversion ({rust:?}.into())"
        );

        let c_to_rust_to_c: CProcfsBase = c_to_rust.into();
        assert_eq!(
            c, c_to_rust_to_c,
            "rust-to-c-to-rust ProcfsBase conversion ({c_to_rust:?}.into())"
        );

        let rust_to_c_to_rust: ProcfsBase = rust_to_c
            .try_into()
            .expect("must be valid value when round-tripping");
        assert_eq!(
            rust, rust_to_c_to_rust,
            "rust-to-c-to-rust ProcfsBase conversion ({rust_to_c:?}.try_into())"
        );
    }

    #[test]
    fn procfsbase_round_trip_procroot() {
        check_round_trip(ProcfsBase::ProcRoot, CProcfsBase::PATHRS_PROC_ROOT);
    }

    #[test]
    fn procfsbase_round_trip_procself() {
        check_round_trip(ProcfsBase::ProcSelf, CProcfsBase::PATHRS_PROC_SELF);
    }

    #[test]
    fn procfsbase_round_trip_procthreadself() {
        check_round_trip(
            ProcfsBase::ProcThreadSelf,
            CProcfsBase::PATHRS_PROC_THREAD_SELF,
        );
    }

    #[test]
    fn procfsbase_round_trip_procpid() {
        check_round_trip(
            ProcfsBase::ProcPid(1),
            CProcfsBase(__PATHRS_PROC_TYPE_PID | 1),
        );
        check_round_trip(
            ProcfsBase::ProcPid(12345),
            CProcfsBase(__PATHRS_PROC_TYPE_PID | 12345),
        );
        check_round_trip(
            ProcfsBase::ProcPid(1122334455),
            CProcfsBase(__PATHRS_PROC_TYPE_PID | 1122334455),
        );
        check_round_trip(
            ProcfsBase::ProcPid(u32::MAX),
            CProcfsBase(__PATHRS_PROC_TYPE_PID | u32::MAX as u64),
        );
    }

    #[test]
    fn pathrs_procfs_open_cached() {
        let procfs_is_cached = ProcfsHandle::new()
            .expect("ProcfsHandle::new should not fail")
            .into_owned_fd()
            .is_none();

        let how = ProcfsOpenHow::default();
        let fd = unsafe { pathrs_procfs_open(&how as *const _, mem::size_of::<ProcfsOpenHow>()) };

        let procfs = if procfs_is_cached {
            assert_eq!(
                fd,
                PATHRS_PROC_DEFAULT_ROOTFD.as_raw_fd(),
                "if ProcfsHandle::new() is cached then pathrs_procfs_open() should return PATHRS_PROC_DEFAULT_ROOTFD",
            );
            ProcfsHandle::new()
        } else {
            ProcfsHandle::try_from_fd(unsafe { OwnedFd::from_raw_fd(fd) })
        }.expect("pathrs_procfs_open should return a valid procfs fd");

        let _ = procfs
            .open(ProcfsBase::ProcSelf, ".", OpenFlags::O_PATH)
            .expect("open(.) should always succeed");
    }

    #[test]
    fn pathrs_procfs_open_unmasked() {
        let how = ProcfsOpenHow {
            flags: ProcfsOpenFlags::PATHRS_PROCFS_NEW_UNMASKED,
        };

        let fd = unsafe { pathrs_procfs_open(&how as *const _, mem::size_of::<ProcfsOpenHow>()) };
        assert!(fd >= 0, "fd value {fd:#x} should be >= 0");

        let procfs = ProcfsHandle::try_from_fd(unsafe { OwnedFd::from_raw_fd(fd) })
            .expect("pathrs_procfs_open should return a valid procfs fd");

        let _ = procfs
            .open(ProcfsBase::ProcSelf, ".", OpenFlags::O_PATH)
            .expect("open(.) should always succeed");
    }

    #[test]
    fn pathrs_procfs_open_bad_flag() {
        let how_bad_flags = ProcfsOpenHow {
            flags: ProcfsOpenFlags::from_bits_retain(0xF000),
        };

        let ret = unsafe {
            pathrs_procfs_open(&how_bad_flags as *const _, mem::size_of::<ProcfsOpenHow>())
        };
        assert!(
            ret < capi_error::__PATHRS_MAX_ERR_VALUE,
            "ret value {ret:#x} should be error value"
        );
        {
            let err = unsafe {
                capi_error::pathrs_errorinfo(ret)
                    .expect("error must be retrievable")
                    .unleak()
            };
            // err.kind() == ErrorKind::InvalidArgument
            assert_eq!(
                err.saved_errno,
                libc::EINVAL as _,
                "invalid flag should return EINVAL"
            );
        }
    }

    #[test]
    fn pathrs_procfs_open_bad_struct() {
        #[repr(C)]
        #[derive(Default, Debug, Clone, Copy, Pod, Zeroable)]
        struct ProcfsOpenHowV2 {
            inner: ProcfsOpenHow,
            extra: u64,
        }

        let how_ok_struct = ProcfsOpenHowV2 {
            inner: ProcfsOpenHow {
                flags: ProcfsOpenFlags::PATHRS_PROCFS_NEW_UNMASKED,
            },
            extra: 0,
        };

        let fd = unsafe {
            pathrs_procfs_open(
                &how_ok_struct as *const _ as *const _,
                mem::size_of::<ProcfsOpenHowV2>(),
            )
        };
        assert!(fd >= 0, "fd value {fd:#x} should be >= 0");
        {
            // Close the file.
            let _ = unsafe { OwnedFd::from_raw_fd(fd) };
        }

        let how_bad_struct = ProcfsOpenHowV2 {
            inner: ProcfsOpenHow {
                flags: ProcfsOpenFlags::PATHRS_PROCFS_NEW_UNMASKED,
            },
            extra: 0xFF,
        };
        let ret = unsafe {
            pathrs_procfs_open(
                &how_bad_struct as *const _ as *const _,
                mem::size_of::<ProcfsOpenHowV2>(),
            )
        };
        assert!(
            ret < capi_error::__PATHRS_MAX_ERR_VALUE,
            "ret value {ret:#x} should be error value"
        );
        {
            let err = unsafe {
                capi_error::pathrs_errorinfo(ret)
                    .expect("error must be retrievable")
                    .unleak()
            };
            // err.kind() == ErrorKind::UnsupportedStructureData
            assert_eq!(
                err.saved_errno,
                libc::E2BIG as _,
                "structure with extra trailing bytes should return E2BIG"
            );
        }
    }
}
