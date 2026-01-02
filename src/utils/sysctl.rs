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
    error::{Error, ErrorExt, ErrorImpl},
    flags::OpenFlags,
    procfs::{ProcfsBase, ProcfsHandle},
};

use std::{
    io::{BufRead, BufReader},
    path::PathBuf,
    str::FromStr,
};

pub(crate) fn sysctl_read_parse<T>(procfs: &ProcfsHandle, sysctl: &str) -> Result<T, Error>
where
    T: FromStr,
    T::Err: Into<ErrorImpl> + Into<Error>,
{
    // "/proc/sys"
    let mut sysctl_path = PathBuf::from("sys");
    // Convert "foo.bar.baz" to "foo/bar/baz".
    sysctl_path.push(sysctl.replace(".", "/"));

    let sysctl_file = procfs.open(ProcfsBase::ProcRoot, sysctl_path, OpenFlags::O_RDONLY)?;

    // Just read the first line.
    let mut reader = BufReader::new(sysctl_file);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|err| ErrorImpl::OsError {
            operation: format!("read first line of {sysctl:?} sysctl").into(),
            source: err,
        })?;

    // Strip newlines.
    line.trim_end_matches("\n")
        .parse()
        .map_err(Error::from)
        .with_wrap(|| {
            format!(
                "could not parse sysctl {sysctl:?} as {:?}",
                std::any::type_name::<T>()
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        error::{Error, ErrorKind},
        procfs::ProcfsHandle,
    };

    use once_cell::sync::Lazy;
    use pretty_assertions::assert_eq;

    // MSRV(1.80): Use LazyLock.
    static TEST_PROCFS_HANDLE: Lazy<ProcfsHandle> =
        Lazy::new(|| ProcfsHandle::new().expect("should be able to get some /proc handle"));

    #[test]
    fn bad_sysctl_file_noexist() {
        assert_eq!(
            sysctl_read_parse::<String>(&TEST_PROCFS_HANDLE, "nonexistent.dummy.sysctl.path")
                .as_ref()
                .map_err(Error::kind),
            Err(ErrorKind::OsError(Some(libc::ENOENT))),
            "reading line from non-existent sysctl",
        );
        assert_eq!(
            sysctl_read_parse::<u32>(&TEST_PROCFS_HANDLE, "nonexistent.sysctl.path")
                .as_ref()
                .map_err(Error::kind),
            Err(ErrorKind::OsError(Some(libc::ENOENT))),
            "parsing line from non-existent sysctl",
        );
    }

    #[test]
    fn bad_sysctl_file_noread() {
        assert_eq!(
            sysctl_read_parse::<String>(&TEST_PROCFS_HANDLE, "vm.drop_caches")
                .as_ref()
                .map_err(Error::kind),
            Err(ErrorKind::OsError(Some(libc::EACCES))),
            "reading line from non-readable sysctl",
        );
        assert_eq!(
            sysctl_read_parse::<u32>(&TEST_PROCFS_HANDLE, "vm.drop_caches")
                .as_ref()
                .map_err(Error::kind),
            Err(ErrorKind::OsError(Some(libc::EACCES))),
            "parse line from non-readable sysctl",
        );
    }

    #[test]
    fn bad_sysctl_parse_invalid_multinumber() {
        assert!(sysctl_read_parse::<String>(&TEST_PROCFS_HANDLE, "kernel.printk").is_ok());
        assert_eq!(
            sysctl_read_parse::<u32>(&TEST_PROCFS_HANDLE, "kernel.printk")
                .as_ref()
                .map_err(Error::kind),
            Err(ErrorKind::InternalError),
            "parsing line from multi-number sysctl",
        );
    }

    #[test]
    fn bad_sysctl_parse_invalid_nonnumber() {
        assert!(sysctl_read_parse::<String>(&TEST_PROCFS_HANDLE, "kernel.random.uuid").is_ok());
        assert_eq!(
            sysctl_read_parse::<u32>(&TEST_PROCFS_HANDLE, "kernel.random.uuid")
                .as_ref()
                .map_err(Error::kind),
            Err(ErrorKind::InternalError),
            "parsing line from non-number sysctl",
        );
    }

    #[test]
    fn sysctl_parse_int() {
        assert!(sysctl_read_parse::<String>(&TEST_PROCFS_HANDLE, "kernel.pid_max").is_ok());
        assert!(sysctl_read_parse::<u64>(&TEST_PROCFS_HANDLE, "kernel.pid_max").is_ok());
    }
}
