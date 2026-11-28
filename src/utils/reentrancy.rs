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

use std::{cell::Cell, thread::LocalKey};

/// Run `f` unless we are already inside `f` (as tracked by `key`) on this
/// thread, in which case return `None`.
///
/// ```ignore
/// fn describe() -> Option<String> {
///     thread_local! {
///         static NESTED_CALL: Cell<bool> = const { Cell::new(false) };
///     }
///     non_reentrant(&NESTED_CALL, || {
///         assert_eq!(describe(), None);
///         "outer".to_string()
///     })
/// }
///
/// assert_eq!(describe(), Some("outer".to_string()));
/// ```
pub(crate) fn non_reentrant<T>(
    key: &'static LocalKey<Cell<bool>>,
    f: impl FnOnce() -> T,
) -> Option<T> {
    struct Reset(&'static LocalKey<Cell<bool>>);
    impl Drop for Reset {
        fn drop(&mut self) {
            self.0.with(|c| c.set(false));
        }
    }
    if key.with(|c| c.replace(true)) {
        return None; // nested
    }
    let _reset = Reset(key);
    Some(f())
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{panic, thread};

    use pretty_assertions::assert_eq;

    #[test]
    fn non_reentrant_basic() {
        thread_local! {
            static KEY: Cell<bool> = const { Cell::new(false) };
        }

        assert_eq!(
            non_reentrant(&KEY, || 42),
            Some(42),
            "non-nested call should run f"
        );
        assert!(
            !KEY.with(Cell::get),
            "guard should be reset after f returns"
        );
        assert_eq!(
            non_reentrant(&KEY, || "again"),
            Some("again"),
            "subsequent non-nested call should run f again"
        );
    }

    #[test]
    fn non_reentrant_nested() {
        thread_local! {
            static KEY: Cell<bool> = const { Cell::new(false) };
        }

        let ret = non_reentrant(&KEY, || {
            assert!(KEY.with(Cell::get), "guard should be set inside f");
            assert_eq!(
                non_reentrant(&KEY, || unreachable!("nested f must not be run")),
                None::<()>,
                "nested call should return None"
            );
            assert!(
                KEY.with(Cell::get),
                "nested call must not reset the outer guard"
            );
            "outer"
        });
        assert_eq!(ret, Some("outer"), "outer call should run f");
        assert!(!KEY.with(Cell::get), "guard should be reset after outer f");
    }

    #[test]
    fn non_reentrant_recursion_bounded() {
        thread_local! {
            static KEY: Cell<bool> = const { Cell::new(false) };
        }

        // Without the guard this would recurse forever.
        fn recurse(calls: &Cell<usize>) -> Option<()> {
            calls.set(calls.get() + 1);
            non_reentrant(&KEY, || {
                recurse(calls);
            })
        }

        let calls = Cell::new(0);
        assert_eq!(recurse(&calls), Some(()), "outer recursive call");
        assert_eq!(
            calls.get(),
            2,
            "only the outer call and one nested attempt should happen"
        );
    }

    #[test]
    fn non_reentrant_panic_resets() {
        thread_local! {
            static KEY: Cell<bool> = const { Cell::new(false) };
        }

        let res = panic::catch_unwind(|| non_reentrant(&KEY, || panic!("boom")));
        assert!(res.is_err(), "panic inside f should propagate");
        assert!(
            !KEY.with(Cell::get),
            "guard should be reset after f unwinds"
        );
        assert_eq!(
            non_reentrant(&KEY, || 1),
            Some(1),
            "call after a panic should run f"
        );
    }

    #[test]
    fn non_reentrant_per_thread() {
        thread_local! {
            static KEY: Cell<bool> = const { Cell::new(false) };
        }

        let ret = non_reentrant(&KEY, || {
            thread::spawn(|| non_reentrant(&KEY, || "other thread"))
                .join()
                .expect("join spawned thread")
        });
        assert_eq!(
            ret,
            Some(Some("other thread")),
            "guard on one thread should not affect other threads"
        );
    }

    #[test]
    fn non_reentrant_independent_keys() {
        thread_local! {
            static KEY_A: Cell<bool> = const { Cell::new(false) };
            static KEY_B: Cell<bool> = const { Cell::new(false) };
        }

        let ret = non_reentrant(&KEY_A, || {
            (non_reentrant(&KEY_A, || "a"), non_reentrant(&KEY_B, || "b"))
        });
        assert_eq!(
            ret,
            Some((None, Some("b"))),
            "guards with different keys should be independent"
        );
    }
}
