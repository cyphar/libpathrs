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

use crate::syscalls;

use std::panic::{self, AssertUnwindSafe};

use libtest_mimic_collect::libtest_mimic::Outcome;

#[derive(Debug, Clone)]
pub(crate) struct SkipTest(pub(crate) Option<String>);

impl SkipTest {
    pub(crate) fn skip(&self) {
        panic::panic_any(self.clone());
    }

    fn to_outcome(&self) -> Outcome {
        Outcome::RuntimeIgnored {
            reason: self.0.clone(),
        }
    }
}

fn run_skippable(test_func: impl FnOnce() -> Outcome + Send) -> Outcome {
    match panic::catch_unwind(move || test_func()) {
        Ok(outcome) => outcome,
        Err(panic) => {
            if let Some(skip) = panic.downcast_ref::<SkipTest>() {
                return skip.into_outcome();
            }
            // Re-reaise the panic if this wasn't a special value.
            panic::panic_any(panic);
        }
    }
}

pub(crate) fn skip_now(msg: String) {
    SkipTest(Some(msg)).skip()
}

macro_rules! skip_if {
    ($cond:expr) => {
        if $cond {
            $crate::tests::common::SkipTest(None).skip();
        }
    };
    ($cond:expr, $($trailing:tt)+) => {
        if $cond {
            $crate::tests::common::SkipTest(Some(format!($($trailing)*).to_string())).skip();
        }
    };
}

pub(crate) fn skip_if_root() {
    skip_if!(syscalls::geteuid() != 0, "Test must be run as root");
}

pub(crate) fn skip_if_not_root() {
    skip_if!(syscalls::geteuid() == 0, "Test must not be run as root");
}
