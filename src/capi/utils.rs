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

use crate::error::{Error, ErrorImpl};

use std::{
    any,
    cmp::{self, Ordering},
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

use bytemuck::{NoUninit, Pod};
use libc::{c_char, c_int, size_t};

/// Generate `.symver` entries for a given function.
///
/// On platforms without stabilised `global_asm!` implementations, this macro is
/// a no-op but will eventually work once they are supported.
///
/// ```ignore
/// # use pathrs::capi::utils::symver;
/// // Create a symbol version with the same name.
/// #[no_mangle]
/// fn foo_bar(a: u64) -> u64 { a + 32 }
/// symver!{
///     fn foo_bar <- (foo_bar, version = "LIBFOO_2.0", default);
/// }
/// // Create a compatibility symbol with a different name. In this case, you
/// // should actually name the implementation function something like __foo_v1
/// // because the symbol will still be public (still unclear why...).
/// fn __foo_bar_v1(a: u64) -> u64 { a + 16 }
/// symver!{
///     #[cfg(feature = "v1_compat")] // meta attributes work
///     fn __foo_bar_v1 <- (foo_bar, version = "LIBFOO_1.0");
/// }
/// ```
macro_rules! symver {
    () => {};
    (@with-meta $(#[$meta:meta])* $($block:tt)+) => {
        // Only generate .symver entries for cdylib.
        #[cfg(cdylib)]
        // Some architectures still have unstable ASM, which stops us from
        // injecting the ".symver" section. You can see the list in
        // LoweringContext::lower_inline_asm (compiler/rustc_asm_lowering).
        #[cfg(any(
            target_arch = "arm",
            target_arch = "aarch64",
            target_arch = "x86",
            target_arch = "x86_64",
            target_arch = "riscv32",
            target_arch = "riscv64",
            // These are only supported after our MSRV and have corresponding
            // #[rustversion::since(1.XY)] tags below.
            target_arch = "loongarch64",
            target_arch = "arm64ec",
            target_arch = "s390x",
            target_arch = "loongarch32",
            // TODO: Once stabilised, add these arches:
            //target_arch = "powerpc",
            //target_arch = "powerpc64",
            //target_arch = "sparc64",
        ))]
        #[cfg_attr(target_arch = "loongarch64", ::rustversion::since(1.72))]
        #[cfg_attr(target_arch = "arm64ec", ::rustversion::since(1.84))]
        #[cfg_attr(target_arch = "s390x", ::rustversion::since(1.84))]
        #[cfg_attr(target_arch = "loongarch32", ::rustversion::since(1.91))]
        $(#[$meta])*
        $($block)*
    };
    ($(#[$meta:meta])* fn $implsym:ident <- ($symname:ident, version = $version:literal); $($tail:tt)*) => {
        $crate::capi::utils::symver! {
            @with-meta $(#[$meta])*
            // .symver $implsym, $symname@$version
            ::std::arch::global_asm! {concat!(
                ".symver ",
                stringify!($implsym),
                ", ",
                stringify!($symname),
                "@",
                $version,
            )}
        }
        $crate::capi::utils::symver! { $($tail)* }
    };
    ($(#[$meta:meta])* fn $implsym:ident <- ($symname:ident, version = $version:literal, default); $($tail:tt)*) => {
        $crate::capi::utils::symver! {
            @with-meta $(#[$meta])*
            // .symver $implsym, $symname@@$version
            ::std::arch::global_asm! {concat!(
                ".symver ",
                stringify!($implsym),
                ", ",
                stringify!($symname),
                "@@",
                $version,
            )}
        }
        $crate::capi::utils::symver! { $($tail)* }
    };
}
pub(crate) use symver;

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

impl<'fd> AsRawFd for CBorrowedFd<'fd> {
    fn as_raw_fd(&self) -> RawFd {
        self.inner
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

/// Copy a `repr(C)` extensible structure to a buffer owned by a C caller. If
/// the C caller's buffer is larger than the library type, the trailing bytes
/// will be zero-filled. If the library type is larger than the C callers buffer
/// and the trailing portion contains non-zero bytes, `Ok(true)` is returned to
/// indicate the data was truncated.
pub(crate) unsafe fn copy_into_extensible_struct<T: NoUninit>(
    dst: *mut T,
    dst_size: usize,
    src: &T,
) -> Result<bool, Error> {
    let raw_src = bytemuck::bytes_of(src);

    if dst.is_null() {
        if dst_size > 0 {
            Err(ErrorImpl::InvalidArgument {
                name: "dst".into(),
                description: "cannot be NULL with non-zero buffer size".into(),
            })?
        }
        // Pretend we "truncated" at size 0.
        return Ok(raw_src.iter().any(|&b| b != 0u8));
    }

    // SAFETY: The C caller guarantees that dst is from a single allocation, is
    // aligned for a u8 slice (generally true for all arrays) and is at least
    // dst_size bytes in length.
    let dst_buffer = unsafe { slice::from_raw_parts_mut(dst as *mut u8, dst_size) };
    let struct_size = mem::size_of::<T>();

    let to_copy = cmp::min(dst_size, struct_size);
    let rest = to_copy..;

    // Copy the common bytes.
    dst_buffer[..to_copy].copy_from_slice(&raw_src[..to_copy]);

    Ok(match dst_size.cmp(&struct_size) {
        // (dst_size < struct_size)
        Ordering::Less => {
            // Were there any non-zero bytes we just truncated?
            raw_src[rest].iter().any(|&b| b != 0u8)
        }
        // (dst_size > struct_size)
        Ordering::Greater => {
            // Clear any trailing bytes.
            bytemuck::fill_zeroes(&mut dst_buffer[rest]);
            false
        }
        // (dst_size == struct_size)
        Ordering::Equal => false, // nothing truncated
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

    #[derive(PartialEq, Eq, Default, Debug, Clone, Copy, Pod, Zeroable)]
    #[repr(C)]
    struct Struct {
        foo: u64,
        bar: u32,
        baz: u32,
    }

    /// Wrap a structure and ensure that operations on it do not write outside
    /// of its bounds. This is checked on `Drop` but can also be manually
    /// checked with [`assert`][Self::assert].
    #[derive(Debug, Clone)]
    #[repr(C)]
    struct Guarded<T> {
        guard_before: [u8; 16],
        data: T,
        guard_after: [u8; 16],
    }

    impl<T> Guarded<T> {
        const GUARD_DATA: [u8; 16] = [0xa5; 16];

        fn new(data: T) -> Self {
            Self {
                guard_before: Self::GUARD_DATA,
                data,
                guard_after: Self::GUARD_DATA,
            }
        }

        fn assert(&self) {
            assert_eq!(
                self.guard_before,
                Self::GUARD_DATA,
                "guard_before was modified",
            );
            assert_eq!(
                self.guard_after,
                Self::GUARD_DATA,
                "guard_after was modified",
            );
        }
    }

    impl<T> Drop for Guarded<T> {
        fn drop(&mut self) {
            // Don't mask an existing panic.
            if !std::thread::panicking() {
                self.assert();
            }
        }
    }

    #[test]
    fn from_extensible_struct() {
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
    fn from_extensible_struct_short() {
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
    fn from_extensible_struct_long() {
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

    #[test]
    fn into_extensible_struct() {
        let example = Struct {
            foo: 0xdeadbeeff00dcafe,
            bar: 0x01234567,
            baz: 0x89abcdef,
        };

        let mut guarded = Guarded::new(Struct::default());
        assert_eq!(
            unsafe {
                copy_into_extensible_struct(
                    &mut guarded.data as *mut Struct,
                    mem::size_of::<Struct>(),
                    &example,
                )
            }
            .expect("copy_into_extensible_struct with dst_size=sizeof(struct)"),
            false,
            "copy_into_extensible_struct(struct, sizeof(struct)) should not be truncated",
        );
        assert_eq!(
            guarded.data, example,
            "copy_into_extensible_struct(struct, sizeof(struct)) should write the full struct",
        );
    }

    #[test]
    fn into_extensible_struct_short() {
        let example = Struct {
            foo: 0xdeadbeeff00dcafe,
            bar: 0x01234567,
            baz: 0x89abcdef,
        };
        let sentinel_dst = Struct {
            foo: 0xaaaaaaaaaaaaaaaa,
            bar: 0xbbbbbbbb,
            baz: 0xcccccccc,
        };

        // Short size (one field) with non-zero trailing data should not
        // overwrite later fields and returns Ok(true) to indicate that non-zero
        // data was truncated.
        let mut guarded = Guarded::new(sentinel_dst);
        assert_eq!(
            unsafe {
                copy_into_extensible_struct(
                    &mut guarded.data as *mut Struct,
                    bytemuck::offset_of!(Struct, bar),
                    &example,
                )
            }
            .expect("copy_into_extensible_struct with dst_size=offsetof(struct, bar)"),
            true,
            "copy_into_extensible_struct(struct, offsetof(struct, bar)) with non-zero src trailing should be truncated",
        );
        assert_eq!(
            guarded.data,
            Struct {
                foo: example.foo,
                bar: sentinel_dst.bar,
                baz: sentinel_dst.baz,
            },
            "copy_into_extensible_struct(struct, offsetof(struct, bar)) should only write foo, leaving trailing dst bytes untouched",
        );
        guarded.assert();

        // Short size (two fields) with non-zero trailing data should not
        // overwrite later fields and returns Ok(true) to indicate that non-zero
        // data was truncated.
        let mut guarded = Guarded::new(sentinel_dst);
        assert_eq!(
            unsafe {
                copy_into_extensible_struct(
                    &mut guarded.data as *mut Struct,
                    bytemuck::offset_of!(Struct, baz),
                    &example,
                )
            }
            .expect("copy_into_extensible_struct with dst_size=offsetof(struct.baz)"),
            true,
            "copy_into_extensible_struct(struct, offsetof(struct.baz)) with non-zero src trailing should be truncated",
        );
        assert_eq!(
            guarded.data,
            Struct {
                foo: example.foo,
                bar: example.bar,
                baz: sentinel_dst.baz,
            },
            "copy_into_extensible_struct(struct, offsetof(struct.baz)) should only write foo and bar, leaving trailing dst bytes untouched",
        );
        guarded.assert();

        // Short size (one fields) with zero trailing data should not overwrite
        // later fields and returns Ok(false) to indicate that non-zero data was
        // *not* truncated.
        let example_zero_trailing = Struct {
            foo: 0xdeadbeeff00dcafe,
            bar: 0,
            baz: 0,
        };
        let mut guarded = Guarded::new(sentinel_dst);
        assert_eq!(
            unsafe {
                copy_into_extensible_struct(
                    &mut guarded.data as *mut Struct,
                    bytemuck::offset_of!(Struct, bar),
                    &example_zero_trailing,
                )
            }
            .expect("copy_into_extensible_struct with dst_size=offsetof(struct, bar) and zero-trailing src"),
            false,
            "copy_into_extensible_struct(struct, offsetof(struct, bar)) with zero-trailing src should not be truncated",
        );
        assert_eq!(
            guarded.data,
            Struct {
                foo: example_zero_trailing.foo,
                bar: sentinel_dst.bar,
                baz: sentinel_dst.baz,
            },
            "copy_into_extensible_struct(struct, offsetof(struct, bar)) with zero-trailing src should write foo only",
        );
        guarded.assert();
    }

    #[test]
    fn into_extensible_struct_long() {
        #[repr(C)]
        #[derive(PartialEq, Eq, Default, Debug, Clone, Copy, Pod, Zeroable)]
        struct StructV2 {
            inner: Struct,
            extra: u64,
        }

        let example = Struct {
            foo: 0xdeadbeeff00dcafe,
            bar: 0x01234567,
            baz: 0x89abcdef,
        };

        // Copy to a larger buffer provided by the user will cause the trailing
        // data (StructV2.extra) to get zeroed.
        let mut guarded = Guarded::new(StructV2 {
            inner: Struct {
                foo: 0xaaaaaaaaaaaaaaaa,
                bar: 0xbbbbbbbb,
                baz: 0xcccccccc,
            },
            extra: 0xffffffffffffffff,
        });
        assert_eq!(
            unsafe {
                copy_into_extensible_struct(
                    &mut guarded.data as *mut StructV2 as *mut Struct,
                    mem::size_of::<StructV2>(),
                    &example,
                )
            }
            .expect("copy_into_extensible_struct with dst_size=sizeof(StructV2)"),
            false,
            "copy_into_extensible_struct(struct, sizeof(StructV2)) should not be truncated",
        );
        assert_eq!(
            guarded.data,
            StructV2 {
                inner: example,
                extra: 0,
            },
            "copy_into_extensible_struct(struct, sizeof(StructV2)) should write src and zero-fill trailing dst bytes",
        );
    }

    #[test]
    fn into_extensible_struct_zerosize() {
        let example = Struct {
            foo: 0xdeadbeeff00dcafe,
            bar: 0x01234567,
            baz: 0x89abcdef,
        };
        let sentinel_dst = Struct {
            foo: 0xaaaaaaaaaaaaaaaa,
            bar: 0xbbbbbbbb,
            baz: 0xcccccccc,
        };

        // Zero size with non-zero struct should write nothing to the struct,
        // and act as though it was truncated (i.e., return Ok(true)).
        let mut guarded = Guarded::new(sentinel_dst);
        assert_eq!(
            unsafe { copy_into_extensible_struct(&mut guarded.data as *mut Struct, 0, &example) }
                .expect("copy_into_extensible_struct with dst_size=0"),
            true,
            "copy_into_extensible_struct(struct, 0) with non-zero src should be truncated",
        );
        assert_eq!(
            guarded.data, sentinel_dst,
            "copy_into_extensible_struct(struct, 0) should not modify dst",
        );
        guarded.assert();

        // Zero size with all-zero struct should write nothing to the struct,
        // and act as though it was *not* truncated (i.e., return Ok(false)).
        let mut guarded = Guarded::new(sentinel_dst);
        assert_eq!(
            unsafe {
                copy_into_extensible_struct(&mut guarded.data as *mut Struct, 0, &Struct::default())
            }
            .expect("copy_into_extensible_struct with dst_size=0 and all-zero src"),
            false,
            "copy_into_extensible_struct(zero-struct, 0) should not be truncated",
        );
        assert_eq!(
            guarded.data, sentinel_dst,
            "copy_into_extensible_struct(zero-struct, 0) should not modify dst",
        );
        guarded.assert();
    }

    #[test]
    fn into_extensible_struct_null() {
        let example = Struct {
            foo: 0xdeadbeeff00dcafe,
            bar: 0x01234567,
            baz: 0x89abcdef,
        };

        // NULL with zero size acts like zero size with any pointer value --
        // "write" nothing to NULL and act as though it was truncated if the
        // data was non-zero (i.e., return Ok(true)).
        assert_eq!(
            unsafe { copy_into_extensible_struct(ptr::null_mut::<Struct>(), 0, &example) }
                .expect("copy_into_extensible_struct with NULL dst and dst_size=0"),
            true,
            "copy_into_extensible_struct(NULL, 0) should return Ok(true) for non-zero struct",
        );
        assert_eq!(
            unsafe {
                copy_into_extensible_struct(ptr::null_mut::<Struct>(), 0, &Struct::default())
            }
            .expect("copy_into_extensible_struct with NULL dst and dst_size=0"),
            false,
            "copy_into_extensible_struct(NULL, 0) should return Ok(false) for zero struct",
        );

        // NULL with non-zero size is an error.
        assert_eq!(
            unsafe {
                copy_into_extensible_struct(
                    ptr::null_mut::<Struct>(),
                    mem::size_of::<Struct>(),
                    &example,
                )
            }
            .map_err(|err| err.kind()),
            Err(ErrorKind::InvalidArgument),
            "copy_into_extensible_struct(NULL, sizeof(struct)) should be InvalidArgument",
        );
    }
}
