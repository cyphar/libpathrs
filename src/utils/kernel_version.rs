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

use std::{
    cmp::{self, Ordering},
    fmt,
};

use once_cell::sync::Lazy;
use rustix::system as rustix_system;

/// A representation of a Linux kernel version that can be mutually compared
/// against, usually to check if a kernel aught to have a particular feature.
///
/// Use [`parse_kernel_version`] to convert a kernel version string (such as
/// returned by `uname -r`) to a [`KernelVersion`].
///
/// # Comparisons #
///
/// Note that the system for comparing kernel versions is not very akin to
/// SemVer because Linux kernel versions can (in principle) have arbitrarily
/// many dot components. If both kernel versions have the same number of
/// components, then the comparison is done left-to-right per-component in a
/// manner identical to the way [`slice`]s are compared. If the kernel versions
/// have different numbers of components then the comparison is done as though
/// the shorter kernel version was right-padded with additional `0` components.
///
/// Thus, `3[.0.0] < 3.1[.0] < 3.1.18 < 4[.0.0]`.
#[derive(Clone, Debug)]
pub(crate) struct KernelVersion(pub(crate) Vec<u64>);

impl fmt::Display for KernelVersion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "linux-{}",
            self.0
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(".")
        )
    }
}

impl Ord for KernelVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        // In contrast to regular slices, when comparing kernel versions we
        // effectively zero-extend the trailing part of the shorter kernel
        // version specification (so 3.1.2 > 3.1[.0] > 3[.0.0]).
        let common_len = cmp::min(self.0.len(), other.0.len());
        match self.0[..common_len].cmp(&other.0[..common_len]) {
            // We only need to deal with the annoying trailing end case if the
            // common part is equal.
            Ordering::Equal => match (
                self.0[common_len..].iter().any(|&n| n > 0),
                other.0[common_len..].iter().any(|&n| n > 0),
            ) {
                (false, false) => Ordering::Equal, // all zeros or equal lengths
                (true, false) => Ordering::Greater, // self tail > 0
                (false, true) => Ordering::Less, // other tail > 0
                (true, true) => unreachable!("both KernelVersion slices cannot have non-zero tails because one must be empty"),
            },
            cmp => cmp,
        }
    }
}

impl PartialOrd for KernelVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for KernelVersion {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for KernelVersion {}

/// Parse a kernel version string like `"4.10.23-1-default"` into a
/// [`KernelVersion`] that can be used for comparisons.
fn parse_kernel_version(kver_str: &str) -> Option<KernelVersion> {
    // Strip off any of the non-version suffixes from the kver string.
    let kver_str = &kver_str[..kver_str
        .find(|ch: char| !ch.is_ascii_digit() && ch != '.')
        .unwrap_or(kver_str.len())];

    let kver = kver_str
        .split('.')
        .map(|num| match num {
            "" => None,            // version components must be non-empty
            _ => num.parse().ok(), // version components must be valid numbers
        })
        .collect::<Option<Vec<_>>>()
        .map(KernelVersion);

    match kver {
        // Versions must have >= 2 components (actually, in practice it's >= 3).
        Some(KernelVersion(ref v)) if v.len() >= 2 => kver,
        _ => None,
    }
}

// MSRV(1.80): Use LazyLock.
pub(crate) static HOST_KERNEL_VERSION: Lazy<KernelVersion> = Lazy::new(host_kernel_version);

pub(crate) fn host_kernel_version() -> KernelVersion {
    parse_kernel_version(&rustix_system::uname().release().to_string_lossy())
        .expect("uname kernel release must be a valid KernelVersion string")
}

/// Returns the result of comparing the kernel version of the running system
/// against the specified kernel version using the specified comparator.
/// `is_kver!(>= 1,2,3)` is equivalent to `is_gte!(1, 2, 3)`.
macro_rules! is_kver {
    ($cmp:tt $($part:literal),+) => {
        {
            $(
                const _: u64 = $part;
            )+

            // Some of our tests rely on using personality(2) to fake older
            // (2.6-era) kernel versions, so we cannot use the cached kernel
            // version when in test builds. However, to make sure that we still
            // test that HOST_KERNEL_VERSION doesn't do anything silly (like
            // crash) we still compute it in our tests.
            let cached_host_kver = &*$crate::utils::kernel_version::HOST_KERNEL_VERSION;
            #[cfg(test)]
            let host_kver = &$crate::utils::kernel_version::host_kernel_version();
            #[cfg(not(test))]
            let host_kver = cached_host_kver;

            #[cfg(test)]
            eprintln!("cached host kernel is {cached_host_kver} but using {host_kver} for test");

            let cmp_kver = &$crate::utils::kernel_version::KernelVersion(vec![$($part),+]);

            host_kver $cmp cmp_kver
        }
    };
}
pub(crate) use is_kver;

/// Returns whether the kernel version of the running system is at least as new
/// as the kernel version specified. See the documentation of [`KernelVersion`]
/// for more information on how kernel versions are compared.
macro_rules! is_gte {
    ($($part:literal),+) => {
        $crate::utils::kernel_version::is_kver!{>= $($part),*}
    };
}
pub(crate) use is_gte;

/// Returns whether the kernel version of the running system is older than the kernel version
/// specified. See the documentation of [`KernelVersion`] for more information on how kernel
/// versions are compared.
macro_rules! is_lt {
    ($($part:literal),+) => {
        $crate::utils::kernel_version::is_kver!{< $($part),*}
    };
}
pub(crate) use is_lt;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syscalls;

    use pretty_assertions::assert_eq;

    macro_rules! kver {
        ($($part:literal),+) => {
            KernelVersion(vec![$($part),+])
        };
    }

    #[test]
    fn parse_kernel_version_bad() {
        assert_eq!(parse_kernel_version(""), None);
        assert_eq!(parse_kernel_version("."), None);
        assert_eq!(parse_kernel_version(".."), None);
        assert_eq!(parse_kernel_version(".-foo"), None);
        assert_eq!(parse_kernel_version("1..3"), None);
        assert_eq!(parse_kernel_version("4...2"), None);

        assert_eq!(parse_kernel_version("a.a.a"), None);
        assert_eq!(parse_kernel_version("invalid"), None);
        assert_eq!(parse_kernel_version("foo"), None);
        assert_eq!(parse_kernel_version("foo.1.3.4"), None);

        assert_eq!(parse_kernel_version("-1.2"), None);
        assert_eq!(parse_kernel_version("+1.2"), None);

        assert_eq!(parse_kernel_version("3a"), None);

        assert_eq!(parse_kernel_version(".1"), None);
        assert_eq!(parse_kernel_version(".1.2"), None);

        assert_eq!(parse_kernel_version("3.foo"), None);
        assert_eq!(parse_kernel_version("42.12."), None);
        assert_eq!(parse_kernel_version("4.10.-default"), None);
        assert_eq!(parse_kernel_version("4.10.-default"), None);
    }

    #[test]
    fn parse_kernel_version_good() {
        assert_eq!(parse_kernel_version("3.7"), Some(kver![3, 7]));
        assert_eq!(parse_kernel_version("3.8"), Some(kver![3, 8]));
        assert_eq!(parse_kernel_version("3.8.0"), Some(kver![3, 8, 0]));
        assert_eq!(parse_kernel_version("3.8.12"), Some(kver![3, 8, 12]));
        assert_eq!(
            parse_kernel_version("3.8.12.10.0.2.5912"),
            Some(kver![3, 8, 12, 10, 0, 2, 5912])
        );
        assert_eq!(
            parse_kernel_version("42.12.1000"),
            Some(kver![42, 12, 1000])
        );
    }

    #[test]
    fn parse_kernel_version_with_suffix() {
        assert_eq!(
            parse_kernel_version("2.6.16.48foobar"),
            Some(kver![2, 6, 16, 48])
        );
        assert_eq!(parse_kernel_version("2.6.16f00b4r"), Some(kver![2, 6, 16]));
        assert_eq!(
            parse_kernel_version("2.6.16.13rc2"),
            Some(kver![2, 6, 16, 13])
        );
        assert_eq!(
            parse_kernel_version("2.6.16.13-rc2"),
            Some(kver![2, 6, 16, 13])
        );
        assert_eq!(
            parse_kernel_version("3.8.16-generic"),
            Some(kver![3, 8, 16])
        );
        assert_eq!(
            parse_kernel_version("6.12.49-1-default"),
            Some(kver![6, 12, 49])
        );
        assert_eq!(
            parse_kernel_version("4.9.27-default-foo.12.23"),
            Some(kver![4, 9, 27])
        );
        assert_eq!(parse_kernel_version("5.15.0+debug"), Some(kver![5, 15, 0]));
        assert_eq!(parse_kernel_version("6.1.0~beta1"), Some(kver![6, 1, 0]));
        assert_eq!(
            parse_kernel_version("5.4.0_custom.1.2"),
            Some(kver![5, 4, 0])
        );
        assert_eq!(parse_kernel_version("3.8-4"), Some(kver![3, 8]));
    }

    #[test]
    fn kernel_version_eq_same_length() {
        assert!(kver![3, 8] == kver![3, 8], "3.8 == 3.8");
        assert!(kver![3, 8, 12] == kver![3, 8, 12], "3.8.12 == 3.8.12");
        assert!(kver![0, 0] == kver![0, 0], "0.0 == 0.0");
        assert!(kver![6, 12, 49] == kver![6, 12, 49], "6.12.49 == 6.12.49");
    }

    #[test]
    fn kernel_version_ne() {
        assert!(kver![3, 8, 0] != kver![3, 8, 1], "3.8 != 3.8.1");
        assert!(kver![3, 8, 12] != kver![4, 8, 12], "3.8.12 != 4.8.12");
        assert!(
            kver![6, 12, 49] != kver![6, 12, 49, 1],
            "6.12.49 != 6.12.49.1"
        );
    }

    #[test]
    fn kernel_version_eq_trailing_zeros() {
        // Trailing zeros should be treated as equal.
        assert!(kver![3, 8] == kver![3, 8, 0], "3.8 == 3.8.0");
        assert!(kver![3, 8] == kver![3, 8, 0, 0], "3.8 == 3.8.0.0");
        assert!(kver![3, 8] == kver![3, 8, 0, 0, 0], "3.8 == 3.8.0.0.0");
        assert!(kver![3, 8, 0] == kver![3, 8, 0, 0], "3.8.0 == 3.8.0.0");
        assert!(kver![3, 8, 0] == kver![3, 8], "3.8.0 == 3.8");
        assert!(kver![5, 0, 0, 0] == kver![5, 0], "5.0.0.0 == 5.0");
    }

    #[test]
    fn kernel_version_lt_same_length() {
        assert!(kver![3, 7] < kver![3, 8], "3.7 < 3.8");
        assert!(kver![3, 8] < kver![4, 0], "3.8 < 4.0");
        assert!(kver![3, 8, 11] < kver![3, 8, 12], "3.8.11 < 3.8.12");
        assert!(kver![2, 6, 32] < kver![3, 0, 0], "2.6.32 < 3.0.0");
        assert!(kver![5, 9, 99] < kver![5, 10, 0], "5.9.99 < 5.10.0");
    }

    #[test]
    fn kernel_version_gt_same_length() {
        assert!(kver![3, 8] > kver![3, 7], "3.8 > 3.7");
        assert!(kver![4, 0] > kver![3, 8], "4.0 > 3.8");
        assert!(kver![3, 8, 12] > kver![3, 8, 11], "3.8.12 > 3.8.11");
        assert!(kver![6, 0, 0] > kver![5, 99, 99], "6.0.0 > 5.99.99");
    }

    #[test]
    fn kernel_version_lt_different_length() {
        // Shorter version is effectively zero-padded.
        assert!(kver![3, 1] < kver![3, 1, 1], "3.1[.0] < 3.1.1");
        assert!(kver![3, 1] < kver![3, 1, 18], "3.1[.0] < 3.1.18");
        assert!(kver![3, 0] < kver![3, 0, 0, 1], "3.0[.0.0] < 3.0.0.1");
        assert!(
            kver![5, 4] < kver![5, 4, 0, 0, 1],
            "5.4[.0.0.0] < 5.4.0.0.1"
        );
    }

    #[test]
    fn kernel_version_gt_different_length() {
        assert!(kver![3, 1, 1] > kver![3, 1], "3.1.1 > 3.1[.0]");
        assert!(kver![3, 1, 18] > kver![3, 1], "3.1.18 > 3.1[.0]");
        assert!(kver![3, 0, 0, 1] > kver![3, 0], "3.0.0.1 > 3.0[.0.0]");
        assert!(
            kver![5, 4, 0, 0, 1] > kver![5, 4],
            "5.4.0.0.1 > 5.4[.0.0.0]"
        );
    }

    #[test]
    fn kernel_version_ordering_chain() {
        // Example from the doc comment.
        assert!(kver![3, 0, 0] < kver![3, 1, 0], "3.0.0 < 3.1.0");
        assert!(kver![3, 1, 0] < kver![3, 1, 18], "3.1.0 < 3.1.18");
        assert!(kver![3, 1, 18] < kver![4, 0, 0], "3.1.18 < 4.0.0");

        // Same example with implicit zeros.
        assert!(kver![3] < kver![3, 1], "3[.0] < 3.1");
        assert!(kver![3, 1] < kver![3, 1, 18], "3.1[.0] < 3.1.18");
        assert!(kver![3, 1, 18] < kver![4, 0], "3.1.18 < 4.0[.0]");
    }

    #[test]
    fn kernel_version_cmp_parsed() {
        // Test comparison through parsed strings, like real kernel versions.
        let v3_8 = parse_kernel_version("3.8.0-generic").expect("parse '3.8.0-generic'");
        let v4_10 = parse_kernel_version("4.10.23-1-default").expect("parse '4.10.23-1-default'");
        let v6_12 = parse_kernel_version("6.12.49-1-default").expect("parse '6.12.49-1-default'");

        assert!(v3_8 < v4_10, "3.8.0 < 4.10.23");
        assert!(v4_10 < v6_12, "4.10.23 < 6.12.49");
        assert!(v3_8 < v6_12, "3.8.0 < 6.12.49");

        // Parsed version with trailing zeros should equal shorter form.
        let v5_4_0 = parse_kernel_version("5.4.0").expect("parse '5.4.0'");
        let v5_4 = parse_kernel_version("5.4").expect("parse '5.4'");
        assert!(v5_4_0 == v5_4, "5.4.0 == 5.4");
    }

    #[test]
    fn kernel_version_gte() {
        let v3_8 = parse_kernel_version("3.8.0-ubuntu.22.04").expect("parse '3.8.0-ubuntu.22.04");

        assert!(kver![3, 8] >= v3_8, "3.8 >= 3.8.0-ubuntu.22.04");
        assert!(kver![3, 8, 1] >= v3_8, "3.8.1 >= 3.8.0-ubuntu.22.04");
        assert!(kver![3, 8, 0, 1] >= v3_8, "3.8.0.1 >= 3.8.0-ubuntu.22.04");
        assert!(kver![4] >= v3_8, "4 >= 3.8.0-ubuntu.22.04");
        assert!(kver![4, 0, 0, 0] >= v3_8, "4.0.0.0 >= 3.8.0-ubuntu.22.04");
        assert!(kver![3, 7, 999] < v3_8, "3.7.999 < 3.8.0-ubuntu.22.04");
    }

    #[test]
    fn kernel_version_display() {
        assert_eq!("linux-2", kver![2].to_string(), "Linux 2");
        assert_eq!(
            "linux-2.6.32.182",
            kver![2, 6, 32, 182].to_string(),
            "Linux 2.6.32.182"
        );
        assert_eq!("linux-3.0", kver![3, 0].to_string(), "Linux 3.0");
        assert_eq!(
            "linux-4.1.18.2",
            kver![4, 1, 18, 2].to_string(),
            "Linux 4.1.18.2"
        );
    }

    #[test]
    fn host_kernel_version_uname26() {
        // The UNAME26 personality lets us fake a pre-3.0 kernel version.
        let _persona_guard = syscalls::scoped_personality(syscalls::PER_UNAME26);

        let host_kver = host_kernel_version();
        assert!(
            kver![3, 0] > host_kver,
            "UNAME26 personality should always result in a <3.0 kernel version: got {host_kver:?}"
        );

        assert!(
            !is_gte!(3, 0),
            "UNAME26 personality should always result in a <3.0 kernel version: is_gte!(3, 0) succeeded"
        );

        assert!(
            is_lt!(3, 0),
            "UNAME26 personality should always result in a <3.0 kernel version: is_lt!(3, 0) failed"
        );

        assert!(
            is_kver!(!= 4, 0),
            "UNAME26 personality should always result in a <3.0 kernel version: is_kver!(!= 4, 0) failed"
        );
    }
}
