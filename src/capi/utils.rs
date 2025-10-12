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

use crate::error::{Error, ErrorImpl};

use std::{
    any, cmp,
    ffi::{CStr, CString, OsStr},
    marker::PhantomData,
    mem,
    os::unix::{
        ffi::OsStrExt,
        io::{AsRawFd, BorrowedFd, RawFd},
    },
    path::Path,
    ptr, slice,
};

use bytemuck::Pod;
use libc::{c_char, c_int, size_t};

/// Equivalent to [`BorrowedFd`], except that there are no restrictions on what
/// value the inner [`RawFd`] can take. This is necessary because C callers
/// could reasonably pass `-1` as a file descriptor value and we need to verify
/// that the value is valid to avoid UB.
///
/// This type is FFI-safe and is intended for use in `extern "C" fn` signatures.
/// While [`BorrowedFd`] (and `Option<BorrowedFd>`) are technically FFI-safe,
/// apparently using them in `extern "C" fn` signatures directly is not
/// recommended for the above reason.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
#[repr(transparent)]
pub struct CBorrowedFd<'fd> {
    inner: RawFd,
    _phantom: PhantomData<BorrowedFd<'fd>>,
}

impl<'fd> CBorrowedFd<'fd> {
    /// Construct a [`CBorrowedFd`] in a `const`-friendly context.
    ///
    /// # Safety
    /// The caller guarantees that the file descriptor will live long enough for
    /// the returned lifetime `'fd`.
    pub(crate) const unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self {
            inner: fd,
            _phantom: PhantomData,
        }
    }

    /// Take a [`CBorrowedFd`] from C FFI and convert it to a proper
    /// [`BorrowedFd`] after making sure that it has a valid value (ie. is not
    /// negative).
    pub(crate) fn try_as_borrowed_fd(&self) -> Result<BorrowedFd<'fd>, Error> {
        // TODO: We might want to support AT_FDCWD in the future. The
        //       openat2 resolver handles it correctly, but the O_PATH
        //       resolver and try_clone() probably need some work.
        // MSRV(1.66): Use match ..0?
        if self.inner.is_negative() {
            Err(ErrorImpl::InvalidArgument {
                // TODO: Should this error be EBADF?
                name: "fd".into(),
                description: "passed file descriptors must not be negative".into(),
            }
            .into())
        } else {
            // SAFETY: The C caller guarantees that the file descriptor is valid for
            //         the lifetime of CBorrowedFd (which is the same lifetime as
            //         BorrowedFd). We verify that the file descriptor is not
            //         negative, so it is definitely valid.
            Ok(unsafe { BorrowedFd::borrow_raw(self.inner) })
        }
    }
}

impl<'fd> From<BorrowedFd<'fd>> for CBorrowedFd<'fd> {
    fn from(fd: BorrowedFd<'_>) -> CBorrowedFd<'_> {
        CBorrowedFd {
            inner: fd.as_raw_fd(),
            _phantom: PhantomData,
        }
    }
}

// TODO: An AsFd impl would be even nicer but I suspect the lifetimes can't be
//       expressed.

pub(crate) unsafe fn parse_path<'a>(path: *const c_char) -> Result<&'a Path, Error> {
    if path.is_null() {
        Err(ErrorImpl::InvalidArgument {
            name: "path".into(),
            description: "cannot be NULL".into(),
        })?
    }
    // SAFETY: C caller guarantees that the path is a valid C-style string.
    let bytes = unsafe { CStr::from_ptr(path) }.to_bytes();
    Ok(OsStr::from_bytes(bytes).as_ref())
}

pub(crate) unsafe fn copy_path_into_buffer(
    path: impl AsRef<Path>,
    buf: *mut c_char,
    bufsize: size_t,
) -> Result<c_int, Error> {
    let path = CString::new(path.as_ref().as_os_str().as_bytes())
        .expect("link from readlink should not contain any nulls");
    // MSRV(1.79): Switch to .count_bytes().
    let path_len = path.to_bytes().len();

    // If the linkbuf is null, we just return the number of bytes we
    // would've written.
    if !buf.is_null() && bufsize > 0 {
        // SAFETY: The C caller guarantees that buf is safe to write to
        // up to bufsize bytes.
        unsafe {
            let to_copy = cmp::min(path_len, bufsize);
            ptr::copy_nonoverlapping(path.as_ptr(), buf, to_copy);
        }
    }
    Ok(path_len as c_int)
}

pub(crate) unsafe fn copy_from_extensible_struct<T: Pod>(
    ptr: *const T,
    size: usize,
) -> Result<T, Error> {
    // SAFETY: The C caller guarantees that ptr is from a single allocation, is
    // aligned for a u8 slice (generally true for all arrays), and is at least
    // size bytes in length.
    let raw_data = unsafe { slice::from_raw_parts(ptr as *const u8, size) };
    let struct_size = mem::size_of::<T>();

    // We may need to make a copy if the structure is smaller than sizeof(T) to
    // zero-pad it, and the Vec needs to live for the rest of this function.
    #[allow(unused_assignments)] // only needed for storage
    let mut struct_data_buf: Option<Vec<u8>> = None;

    // MSRV(1.80): Use slice::split_at_checked()?
    let (struct_data, trailing) = if raw_data.len() >= struct_size {
        raw_data.split_at(struct_size)
    } else {
        let mut buf = vec![0u8; struct_size];
        buf[0..raw_data.len()].copy_from_slice(raw_data);
        struct_data_buf = Some(buf);
        (
            &struct_data_buf
                .as_ref()
                .expect("Option just assigned with Some must contain Some")[..],
            &[][..],
        )
    };
    debug_assert!(
        struct_data.len() == struct_size,
        "copy_from_extensible_struct should compute the struct size correctly"
    );

    // TODO: Can we get an optimised memchr_inv implementation? Unfortunately,
    // see <https://github.com/BurntSushi/memchr/issues/166> -- memchr doesn't
    // have this and is unlikely to have it in the future.
    if trailing.iter().any(|&ch| ch != 0) {
        return Err(ErrorImpl::UnsupportedStructureData {
            name: format!("c struct {}", any::type_name::<T>()).into(),
        }
        .into());
    }

    // NOTE: Even though we have a slice, we can only be sure it's aligned to u8
    // (i.e., any alignment). It's better to just return a copy than error out
    // in the non-aligned case...
    bytemuck::try_pod_read_unaligned(struct_data).map_err(|err| {
        ErrorImpl::BytemuckPodCastError {
            description: format!("cannot cast passed buffer into {}", any::type_name::<T>()).into(),
            source: err,
        }
        .into()
    })
}

pub(crate) trait Leakable: Sized {
    /// Leak a structure such that it can be passed through C-FFI.
    fn leak(self) -> &'static mut Self {
        Box::leak(Box::new(self))
    }

    /// Given a structure leaked through Leakable::leak, un-leak it.
    ///
    /// SAFETY: Callers must be sure to only ever call this once on a given
    /// pointer (otherwise memory corruption will occur).
    unsafe fn unleak(&'static mut self) -> Self {
        // SAFETY: Box::from_raw is safe because the caller guarantees that
        // the pointer we get is the same one we gave them, and it will only
        // ever be called once with the same pointer.
        *unsafe { Box::from_raw(self as *mut Self) }
    }

    /// Shorthand for `std::mem::drop(self.unleak())`.
    ///
    /// SAFETY: Same unsafety issue as `self.unleak()`.
    unsafe fn free(&'static mut self) {
        // SAFETY: Caller guarantees this is safe to do.
        let _ = unsafe { self.unleak() };
        // drop Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::error::ErrorKind;

    use bytemuck::{Pod, Zeroable};
    use pretty_assertions::assert_eq;

    #[repr(C)]
    #[derive(PartialEq, Eq, Default, Debug, Clone, Copy, Pod, Zeroable)]
    struct Struct {
        foo: u64,
        bar: u32,
        baz: u32,
    }

    #[test]
    fn extensible_struct() {
        let example = Struct {
            foo: 0xdeadbeeff00dcafe,
            bar: 0x01234567,
            baz: 0x89abcdef,
        };

        assert_eq!(
            unsafe {
                copy_from_extensible_struct(&example as *const Struct, mem::size_of::<Struct>())
            }
            .expect("copy_from_extensible_struct with size=sizeof(struct)"),
            example,
            "copy_from_extensible_struct(struct, sizeof(struct))",
        );
    }

    #[test]
    fn extensible_struct_short() {
        let example = Struct {
            foo: 0xdeadbeeff00dcafe,
            bar: 0x01234567,
            baz: 0x89abcdef,
        };

        assert_eq!(
            unsafe { copy_from_extensible_struct(&example as *const Struct, 0) }
                .expect("copy_from_extensible_struct with size=0"),
            Struct::default(),
            "copy_from_extensible_struct(struct, 0)",
        );

        assert_eq!(
            unsafe {
                copy_from_extensible_struct(
                    &example as *const Struct,
                    bytemuck::offset_of!(Struct, bar),
                )
            }
            .expect("copy_from_extensible_struct with size=offsetof(struct.bar)"),
            Struct {
                foo: example.foo,
                ..Default::default()
            },
            "copy_from_extensible_struct(struct, offsetof(struct, bar))",
        );

        assert_eq!(
            unsafe {
                copy_from_extensible_struct(
                    &example as *const Struct,
                    bytemuck::offset_of!(Struct, baz),
                )
            }
            .expect("copy_from_extensible_struct with size=offsetof(struct.baz)"),
            Struct {
                foo: example.foo,
                bar: example.bar,
                ..Default::default()
            },
            "copy_from_extensible_struct(struct, offsetof(struct, bar))",
        );
    }

    #[test]
    fn extensible_struct_long() {
        #[repr(C)]
        #[derive(PartialEq, Eq, Default, Debug, Clone, Copy, Pod, Zeroable)]
        struct StructV2 {
            inner: Struct,
            extra: u64,
        }

        let example_compatible = StructV2 {
            inner: Struct {
                foo: 0xdeadbeeff00dcafe,
                bar: 0x01234567,
                baz: 0x89abcdef,
            },
            extra: 0,
        };

        assert_eq!(
            unsafe {
                copy_from_extensible_struct(
                    &example_compatible as *const StructV2 as *const Struct,
                    mem::size_of::<StructV2>(),
                )
            }
            .expect("copy_from_extensible_struct with size=sizeof(structv2)"),
            example_compatible.inner,
            "copy_from_extensible_struct(structv2, sizeof(structv2)) with only trailing zero bytes",
        );

        let example_compatible = StructV2 {
            inner: Struct {
                foo: 0xdeadbeeff00dcafe,
                bar: 0x01234567,
                baz: 0x89abcdef,
            },
            extra: 0x1,
        };

        assert_eq!(
            unsafe {
                copy_from_extensible_struct(
                    &example_compatible as *const StructV2 as *const Struct,
                    mem::size_of::<StructV2>(),
                )
            }
            .map_err(|err| err.kind()),
            Err(ErrorKind::UnsupportedStructureData),
            "copy_from_extensible_struct(structv2, sizeof(structv2)) with trailing non-zero bytes",
        );
    }
}
