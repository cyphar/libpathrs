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

use crate::{
    syscalls,
    utils::{self, RawProcfsRoot},
};

// Awful hack to detect the /proc/sys overmount case in containers.
pub(crate) fn has_proc_sys_overmounts() -> bool {
    let proc_root_mnt_id =
        utils::fetch_mnt_id(RawProcfsRoot::UnsafeGlobal, syscalls::BADFD, "/proc")
            .expect("get mount id of /proc");
    let proc_sys_mnt_id =
        utils::fetch_mnt_id(RawProcfsRoot::UnsafeGlobal, syscalls::BADFD, "/proc/sys")
            .expect("get mount id of /proc/sys");

    proc_root_mnt_id != proc_sys_mnt_id
}

// FIXME(libtest skip): Replace with this a panic-based runtime test skipping
// harness, as this hotfix only works if the top-level #[test] function calls
// this.
macro_rules! hotfix_skip_if_proc_sys_overmounts {
    (return $ret:expr) => {
        if $crate::tests::common::has_proc_sys_overmounts() {
            ::std::eprintln!("/proc/sys has overmounts -- skipping this test.");
            return $ret;
        }
    };
    () => {
        $crate::tests::common::hotfix_skip_if_proc_sys_overmounts!(return Ok(()));
    };
}
pub(crate) use hotfix_skip_if_proc_sys_overmounts;
