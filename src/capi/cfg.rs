/*
 * libpathrs: safe path resolution on Linux
 * Copyright (C) 2019-2024 Aleksa Sarai <cyphar@cyphar.com>
 * Copyright (C) 2019-2024 SUSE LLC
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

use std::{cmp, mem, ptr};

/// ### Safety
///
/// Implementing this trait means the type you are using has the following
/// properties that make it safe to be used as an extensible structure:
///
///  1. The structure is `#[repr(C)]` and is C FFI safe.
///  2. The structure can safely be filled with any bit pattern (including but
///     not limited to `mem::zeroed()`).
///  3. The structure contains no padding (ideally *not* through
///     `#[repr(packed)]` because of the risk of unaligned reads, but instead by
///     making sure that different integer types).
// TODO: Should we use zerocopy traits here instead? The specific semantics we
//       need for copy_struct_from don't really match zerocopy but we could use
//       FromZeros/FromBytes. Then again, we should avoid adding new deps if
//       possible.
unsafe trait ExtensibleStruct: Sized {
    fn zeroed() -> Self {
        // SAFETY: Implementing this trait means this must be safe.
        unsafe { mem::zeroed() }
    }

    fn as_chr_ptr(ptr: *const Self) -> *const u8 {
        // SAFETY: Implementing this trait means that the structure has a
        // consistent [u8] representation.
        ptr as *const u8
    }
}

unsafe fn memchr_inv(needle: u8, haystack: *const u8, size: usize) -> Option<*const u8> {
    debug_assert!(size <= isize::MAX as usize, "size must be valid");
    for idx in 0..=size {
        // SAFETY: The caller guarantees that the buffer is valid for size
        // bytes.
        let ptr = unsafe { haystack.offset(idx as isize) };
        if unsafe { *ptr } != needle {
            return Some(ptr);
        }
    }
    None
}

unsafe fn copy_struct_from<T: ExtensibleStruct>(src: *const T, user_size: usize) -> Option<T> {
    let lib_size = mem::size_of::<T>();
    let size = cmp::min(user_size, lib_size);
    let rest = user_size - size;
    debug_assert!(rest >= 0, "remaining size needs to be non-negative");
    debug_assert!(size + rest == user_size);

    // SAFETY: We only operate within src[0..user_size] here.
    unsafe {
        let mut dst = T::zeroed();
        let ptr = ptr::from_mut(&mut dst) as *mut u8;
        let trailing = ptr.offset(size as isize);
        if memchr_inv(0u8, trailing, rest).is_some() {
            return None;
        }
        ptr::copy_nonoverlapping(T::as_chr_ptr(src), ptr, size);
        Some(dst)
    }
}

#[repr(C)]
struct CConfig {
    flags: u64,
}

// SAFETY: CConfig is #[repr(C)], only contains primitive integer types and is
//         structured to ensure it has no padding.
unsafe impl ExtensibleStruct for CConfig {}
