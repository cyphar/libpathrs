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

use crate::{capi::error as capi_error, error::Error, Handle, Root};

use std::{
    fs::File,
    os::unix::io::{IntoRawFd, OwnedFd},
};

use libc::c_int;

pub(super) type CReturn = c_int;

pub(super) trait IntoCReturn {
    fn into_c_return(self) -> CReturn;
}

// TODO: Is it possible for us to return an actual OwnedFd through FFI when we
//       need to? Unfortunately, CReturn may end up returning -1 which is an
//       invalid OwnedFd/BorrowedFd value...
//       <https://users.rust-lang.org/t/what-is-the-reason-for-the-1-restriction-in-ownedfd-borrowedfd/116450>

impl IntoCReturn for () {
    fn into_c_return(self) -> CReturn {
        0
    }
}

impl IntoCReturn for CReturn {
    fn into_c_return(self) -> CReturn {
        self
    }
}

impl IntoCReturn for OwnedFd {
    fn into_c_return(self) -> CReturn {
        self.into_raw_fd()
    }
}

impl IntoCReturn for Root {
    fn into_c_return(self) -> CReturn {
        OwnedFd::from(self).into_c_return()
    }
}

impl IntoCReturn for Handle {
    fn into_c_return(self) -> CReturn {
        OwnedFd::from(self).into_c_return()
    }
}

impl IntoCReturn for File {
    fn into_c_return(self) -> CReturn {
        OwnedFd::from(self).into_c_return()
    }
}

impl<V> IntoCReturn for Result<V, Error>
where
    V: IntoCReturn,
{
    fn into_c_return(self) -> CReturn {
        // self.map_or_else(store_error, IntoCReturn::into_c_return)
        match self {
            Ok(ok) => ok.into_c_return(),
            Err(err) => capi_error::store_error(err),
        }
    }
}
