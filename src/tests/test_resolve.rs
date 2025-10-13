// SPDX-License-Identifier: LGPL-3.0-or-later
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

#[cfg(feature = "capi")]
use crate::tests::capi::CapiRoot;
use crate::{error::ErrorKind, flags::ResolverFlags, resolvers::ResolverBackend, Root};

use std::path::Path;

use anyhow::Error;

macro_rules! resolve_tests {
    // resolve_tests! {
    //      [create_root_path] {
    //          test_ok: resolve(...) => Ok(("path", libc::S_IF...))
    //          test_err: resolve(...) => Err(ErrorKind::...)
    //      }
    // }
    ([$with_root_fn:ident] $(#[cfg($ignore_meta:meta)])* rust-fn $test_name:ident (mut $root_var:ident : Root) $body:block => $expected:expr) => {
        paste::paste! {
            #[test]
            $(#[cfg_attr(not($ignore_meta), ignore)])*
            fn [<root_ $test_name _default>]() -> Result<(), Error> {
                utils::$with_root_fn(|root_dir: &Path| {
                    let mut $root_var = Root::open(root_dir)?;
                    assert_eq!(
                        $root_var.resolver_backend(),
                        ResolverBackend::default(),
                        "ResolverBackend not the default despite not being configured"
                    );

                    { $body }

                    // Make sure the mut $root_var doesn't give us a warning.
                    $root_var.set_resolver_flags($root_var.resolver_flags());
                    Ok(())
                })
            }

            #[test]
            $(#[cfg_attr(not($ignore_meta), ignore)])*
            fn [<rootref_ $test_name _default>]() -> Result<(), Error> {
                utils::$with_root_fn(|root_dir: &Path| {
                    let root = Root::open(root_dir)?;
                    let mut $root_var = root.as_ref();
                    assert_eq!(
                        $root_var.resolver_backend(),
                        ResolverBackend::default(),
                        "ResolverBackend not the default despite not being configured"
                    );

                    { $body }

                    // Make sure the mut $root_var doesn't give us a warning.
                    $root_var.set_resolver_flags($root_var.resolver_flags());
                    Ok(())
                })
            }

            #[test]
            #[cfg_attr(feature = "_test_enosys_openat2", ignore, allow(unused_attributes))]
            $(#[cfg_attr(not($ignore_meta), ignore)])*
            fn [<root_ $test_name _openat2>]() -> Result<(), Error> {
                utils::$with_root_fn(|root_dir: &Path| {
                    let mut $root_var = Root::open(root_dir)?;
                    $root_var.set_resolver_backend(ResolverBackend::KernelOpenat2);
                    assert_eq!(
                        $root_var.resolver_backend(),
                        ResolverBackend::KernelOpenat2,
                        "incorrect ResolverBackend despite using set_resolver_backend"
                    );

                    if !$root_var.resolver_backend().supported() {
                        // Skip if not supported.
                        return Ok(());
                    }

                    { $body }

                    Ok(())
                })
            }

            #[test]
            #[cfg_attr(feature = "_test_enosys_openat2", ignore, allow(unused_attributes))]
            $(#[cfg_attr(not($ignore_meta), ignore)])*
            fn [<rootref_ $test_name _openat2>]() -> Result<(), Error> {
                utils::$with_root_fn(|root_dir: &Path| {
                    let root = Root::open(root_dir)?;
                    let mut $root_var = root.as_ref();
                    $root_var.set_resolver_backend(ResolverBackend::KernelOpenat2);
                    assert_eq!(
                        $root_var.resolver_backend(),
                        ResolverBackend::KernelOpenat2,
                        "incorrect ResolverBackend despite using set_resolver_backend"
                    );
                    if !$root_var.resolver_backend().supported() {
                        // Skip if not supported.
                        return Ok(());
                    }

                    { $body }

                    Ok(())
                })
            }

            #[test]
            #[cfg_attr(feature = "_test_enosys_openat2", ignore, allow(unused_attributes))]
            $(#[cfg_attr(not($ignore_meta), ignore)])*
            fn [<root_ $test_name _opath>]() -> Result<(), Error> {
                utils::$with_root_fn(|root_dir: &Path| {
                    let mut $root_var = Root::open(root_dir)?;
                    $root_var.set_resolver_backend(ResolverBackend::EmulatedOpath);
                    assert_eq!(
                        $root_var.resolver_backend(),
                        ResolverBackend::EmulatedOpath,
                        "incorrect ResolverBackend despite using set_resolver_backend"
                    );
                    // EmulatedOpath is always supported.
                    assert!(
                        $root_var.resolver_backend().supported(),
                        "emulated opath is always supported",
                    );

                    { $body }

                    Ok(())
                })
            }

            #[test]
            #[cfg_attr(feature = "_test_enosys_openat2", ignore, allow(unused_attributes))]
            $(#[cfg_attr(not($ignore_meta), ignore)])*
            fn [<rootref_ $test_name _opath>]() -> Result<(), Error> {
                utils::$with_root_fn(|root_dir: &Path| {
                    let root = Root::open(root_dir)?;
                    let mut $root_var = root
                        .as_ref();
                    $root_var.set_resolver_backend(ResolverBackend::EmulatedOpath);
                    assert_eq!(
                        $root_var.resolver_backend(),
                        ResolverBackend::EmulatedOpath,
                        "incorrect ResolverBackend despite using set_resolver_backend"
                    );
                    // EmulatedOpath is always supported.
                    assert!(
                        $root_var.resolver_backend().supported(),
                        "emulated opath is always supported",
                    );

                    { $body }

                    // Make sure root_dir is not dropped earlier.
                    let _root_dir = root_dir;
                    Ok(())
                })
            }
        }
    };

    ([$with_root_fn:ident] $(#[cfg($ignore_meta:meta)])* capi-fn $test_name:ident ($root_var:ident : CapiRoot) $body:block => $expected:expr) => {
        paste::paste! {
            #[test]
            #[cfg(feature = "capi")]
            $(#[cfg_attr(not($ignore_meta), ignore)])*
            fn [<capi_root_ $test_name>]() -> Result<(), Error> {
                utils::$with_root_fn(|root_dir: &Path| {
                    let $root_var = CapiRoot::open(root_dir)?;

                    { $body }

                    Ok(())
                })
            }
        }
    };

    ([$with_root_fn:ident] $(#[cfg($ignore_meta:meta)])* @rust-impl $test_name:ident $op_name:ident ($path:expr, $rflags:expr, $no_follow_trailing:expr) => $expected:expr) => {
        paste::paste! {
            resolve_tests! {
                [$with_root_fn]
                $(#[cfg($ignore_meta)])*
                rust-fn [<$op_name _ $test_name>](mut root: Root) {
                    root.set_resolver_flags($rflags);

                    let expected = $expected;
                    utils::[<check_root_ $op_name>](
                        &root,
                        $path,
                        $no_follow_trailing,
                        expected,
                    )?;
                } => $expected
            }
        }
    };

    ([$with_root_fn:ident] $(#[cfg($ignore_meta:meta)])* @capi-impl $test_name:ident $op_name:ident ($path:expr, $no_follow_trailing:expr) => $expected:expr) => {
        paste::paste! {
            resolve_tests! {
                [$with_root_fn]
                $(#[cfg($ignore_meta)])*
                capi-fn [<$op_name _ $test_name>](root: CapiRoot) {
                    let expected = $expected;
                    utils::[<check_root_ $op_name>](
                        &root,
                        $path,
                        $no_follow_trailing,
                        expected,
                    )?;
                } => $expected
            }
        }
    };

    ([$with_root_fn:ident] $(#[cfg($ignore_meta:meta)])* @impl $test_name:ident $op_name:ident ($path:expr, rflags = $($rflag:ident)|+) => $expected:expr ) => {
        resolve_tests! {
            [$with_root_fn]
            $(#[cfg($ignore_meta)])*
            @rust-impl $test_name $op_name($path, $(ResolverFlags::$rflag)|*, false) => $expected
        }
        // The C API doesn't support custom ResolverFlags.
    };

    ([$with_root_fn:ident] $(#[cfg($ignore_meta:meta)])* @impl $test_name:ident $op_name:ident ($path:expr, no_follow_trailing = $no_follow_trailing:expr) => $expected:expr ) => {
        resolve_tests! {
            [$with_root_fn]
            $(#[cfg($ignore_meta)])*
            @rust-impl $test_name $op_name($path, ResolverFlags::empty(), $no_follow_trailing) => $expected
        }
        resolve_tests! {
            [$with_root_fn]
            $(#[cfg($ignore_meta)])*
            @capi-impl $test_name $op_name($path, $no_follow_trailing) => $expected
        }
    };

    ([$with_root_fn:ident] $(#[cfg($ignore_meta:meta)])* @impl $test_name:ident $op_name:ident ($path:expr) => $expected:expr ) => {
        resolve_tests! {
            [$with_root_fn]
            $(#[cfg($ignore_meta)])*
            @rust-impl $test_name $op_name($path, ResolverFlags::empty(), false) => $expected
        }
        resolve_tests! {
            [$with_root_fn]
            $(#[cfg($ignore_meta)])*
            @capi-impl $test_name $op_name($path, false) => $expected
        }
    };

    ($([$with_root_fn:ident] { $($(#[cfg($ignore_meta:meta)])* $test_name:ident : $op_name:ident ($($args:tt)*) => $expected:expr);* $(;)? });* $(;)?) => {
        $( $(
            resolve_tests! {
                [$with_root_fn]
                $(#[cfg($ignore_meta)])*
                @impl $test_name $op_name ($($args)*) => $expected
            }
        )* )*
    }
}

resolve_tests! {
    // Test the magic-link-related handling.
    [with_proc] {
        proc_pseudo_magiclink: resolve("self/sched") => Ok(("{{/proc/self}}/sched", libc::S_IFREG));
        proc_pseudo_magiclink_nosym1: resolve("self", rflags = NO_SYMLINKS) => Err(ErrorKind::OsError(Some(libc::ELOOP)));
        proc_pseudo_magiclink_nosym2: resolve("self/sched", rflags = NO_SYMLINKS) => Err(ErrorKind::OsError(Some(libc::ELOOP)));
        proc_pseudo_magiclink_nofollow1: resolve("self", no_follow_trailing = true) => Ok(("self", libc::S_IFLNK));
        proc_pseudo_magiclink_nofollow2: resolve("self/sched", no_follow_trailing = true) => Ok(("{{/proc/self}}/sched", libc::S_IFREG));

        // Verify forced RESOLVE_NO_MAGICLINKS behaviour.
        proc_magiclink: resolve("self/exe") => Err(ErrorKind::OsError(Some(libc::ELOOP)));
        proc_magiclink_nofollow: resolve("self/exe", no_follow_trailing = true) => Ok(("{{/proc/self}}/exe", libc::S_IFLNK));
        proc_magiclink_component_nofollow: resolve("self/root/etc/passwd", no_follow_trailing = true) => Err(ErrorKind::OsError(Some(libc::ELOOP)));
    };

    // Complete lookups.
    [with_basic_tree] {
        complete_root1: resolve("/") => Ok(("/", libc::S_IFDIR));
        complete_root2: resolve("/../../../../../..") => Ok(("/", libc::S_IFDIR));
        complete_root_link1: resolve("root-link1") => Ok(("/", libc::S_IFDIR));
        complete_root_link2: resolve("root-link2") => Ok(("/", libc::S_IFDIR));
        complete_root_link3: resolve("root-link3") => Ok(("/", libc::S_IFDIR));
        complete_dir1: resolve("a") => Ok(("/a", libc::S_IFDIR));
        complete_dir2: resolve("b/c/d/e/f") => Ok(("/b/c/d/e/f", libc::S_IFDIR));
        complete_dir3: resolve("b///././c////.//d/./././///e////.//./f//././././") => Ok(("/b/c/d/e/f", libc::S_IFDIR));
        complete_file: resolve("b/c/file") => Ok(("/b/c/file", libc::S_IFREG));
        complete_file_link: resolve("b-file") => Ok(("/b/c/file", libc::S_IFREG));
        complete_fifo: resolve("b/fifo") => Ok(("/b/fifo", libc::S_IFIFO));
        complete_sock: resolve("b/sock") => Ok(("/b/sock", libc::S_IFSOCK));
        // Partial lookups.
        partial_dir_basic: resolve("a/b/c/d/e/f/g/h") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        partial_dir_dotdot: resolve("a/foo/../bar/baz") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        // Non-lexical symlinks.
        nonlexical_basic_complete: resolve("target") => Ok(("/target", libc::S_IFDIR));
        nonlexical_basic_complete1: resolve("target/") => Ok(("/target", libc::S_IFDIR));
        nonlexical_basic_complete2: resolve("target//") => Ok(("/target", libc::S_IFDIR));
        nonlexical_basic_partial: resolve("target/foo") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        nonlexical_basic_partial_dotdot: resolve("target/../target/foo/bar/../baz") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        nonlexical_level1_abs_complete1: resolve("link1/target_abs") => Ok(("/target", libc::S_IFDIR));
        nonlexical_level1_abs_complete2: resolve("link1/target_abs/") => Ok(("/target", libc::S_IFDIR));
        nonlexical_level1_abs_complete3: resolve("link1/target_abs//") => Ok(("/target", libc::S_IFDIR));
        nonlexical_level1_abs_partial: resolve("link1/target_abs/foo") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        nonlexical_level1_abs_partial_dotdot: resolve("link1/target_abs/../target/foo/bar/../baz") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        nonlexical_level1_rel_complete1: resolve("link1/target_rel") => Ok(("/target", libc::S_IFDIR));
        nonlexical_level1_rel_complete2: resolve("link1/target_rel/") => Ok(("/target", libc::S_IFDIR));
        nonlexical_level1_rel_complete3: resolve("link1/target_rel//") => Ok(("/target", libc::S_IFDIR));
        nonlexical_level1_rel_partial: resolve("link1/target_rel/foo") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        nonlexical_level1_rel_partial_dotdot: resolve("link1/target_rel/../target/foo/bar/../baz") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        nonlexical_level2_abs_abs_complete1: resolve("link2/link1_abs/target_abs") => Ok(("/target", libc::S_IFDIR));
        nonlexical_level2_abs_abs_complete2: resolve("link2/link1_abs/target_abs/") => Ok(("/target", libc::S_IFDIR));
        nonlexical_level2_abs_abs_complete3: resolve("link2/link1_abs/target_abs//") => Ok(("/target", libc::S_IFDIR));
        nonlexical_level2_abs_abs_partial: resolve("link2/link1_abs/target_abs/foo") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        nonlexical_level2_abs_abs_partial_dotdot: resolve("link2/link1_abs/target_abs/../target/foo/bar/../baz") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        nonlexical_level2_abs_rel_complete1: resolve("link2/link1_abs/target_rel") => Ok(("/target", libc::S_IFDIR));
        nonlexical_level2_abs_rel_complete2: resolve("link2/link1_abs/target_rel/") => Ok(("/target", libc::S_IFDIR));
        nonlexical_level2_abs_rel_complete3: resolve("link2/link1_abs/target_rel//") => Ok(("/target", libc::S_IFDIR));
        nonlexical_level2_abs_rel_partial: resolve("link2/link1_abs/target_rel/foo") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        nonlexical_level2_abs_rel_partial_dotdot: resolve("link2/link1_abs/target_rel/../target/foo/bar/../baz") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        nonlexical_level2_abs_open_complete1: resolve("link2/link1_abs/../target") => Ok(("/target", libc::S_IFDIR));
        nonlexical_level2_abs_open_complete2: resolve("link2/link1_abs/../target/") => Ok(("/target", libc::S_IFDIR));
        nonlexical_level2_abs_open_complete3: resolve("link2/link1_abs/../target//") => Ok(("/target", libc::S_IFDIR));
        nonlexical_level2_abs_open_partial: resolve("link2/link1_abs/../target/foo") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        nonlexical_level2_abs_open_partial_dotdot: resolve("link2/link1_abs/../target/../target/foo/bar/../baz") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        nonlexical_level2_rel_abs_complete1: resolve("link2/link1_rel/target_abs") => Ok(("/target", libc::S_IFDIR));
        nonlexical_level2_rel_abs_complete2: resolve("link2/link1_rel/target_abs/") => Ok(("/target", libc::S_IFDIR));
        nonlexical_level2_rel_abs_complete3: resolve("link2/link1_rel/target_abs//") => Ok(("/target", libc::S_IFDIR));
        nonlexical_level2_rel_abs_partial: resolve("link2/link1_rel/target_abs/foo") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        nonlexical_level2_rel_abs_partial_dotdot: resolve("link2/link1_rel/target_abs/../target/foo/bar/../baz") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        nonlexical_level2_rel_rel_complete1: resolve("link2/link1_rel/target_rel") => Ok(("/target", libc::S_IFDIR));
        nonlexical_level2_rel_rel_complete2: resolve("link2/link1_rel/target_rel/") => Ok(("/target", libc::S_IFDIR));
        nonlexical_level2_rel_rel_complete3: resolve("link2/link1_rel/target_rel//") => Ok(("/target", libc::S_IFDIR));
        nonlexical_level2_rel_rel_partial: resolve("link2/link1_rel/target_rel/foo") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        nonlexical_level2_rel_rel_partial_dotdot: resolve("link2/link1_rel/target_rel/../target/foo/bar/../baz") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        nonlexical_level2_rel_open_complete1: resolve("link2/link1_rel/../target") => Ok(("/target", libc::S_IFDIR));
        nonlexical_level2_rel_open_complete2: resolve("link2/link1_rel/../target/") => Ok(("/target", libc::S_IFDIR));
        nonlexical_level2_rel_open_complete3: resolve("link2/link1_rel/../target//") => Ok(("/target", libc::S_IFDIR));
        nonlexical_level2_rel_open_partial: resolve("link2/link1_rel/../target/foo") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        nonlexical_level2_rel_open_partial_dotdot: resolve("link2/link1_rel/../target/../target/foo/bar/../baz") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        nonlexical_level3_abs_complete1: resolve("link3/target_abs") => Ok(("/target", libc::S_IFDIR));
        nonlexical_level3_abs_complete2: resolve("link3/target_abs/") => Ok(("/target", libc::S_IFDIR));
        nonlexical_level3_abs_complete3: resolve("link3/target_abs//") => Ok(("/target", libc::S_IFDIR));
        nonlexical_level3_abs_partial: resolve("link3/target_abs/foo") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        nonlexical_level3_abs_partial_dotdot: resolve("link3/target_abs/../target/foo/bar/../baz") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        nonlexical_level3_rel_complete: resolve("link3/target_rel") => Ok(("/target", libc::S_IFDIR));
        nonlexical_level3_rel_partial: resolve("link3/target_rel/foo") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        nonlexical_level3_rel_partial_dotdot: resolve("link3/target_rel/../target/foo/bar/../baz") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        // Partial lookups due to hitting a non_directory.
        partial_nondir_slash1: resolve("b/c/file/") => Err(ErrorKind::OsError(Some(libc::ENOTDIR)));
        partial_nondir_slash2: resolve("b/c/file//") => Err(ErrorKind::OsError(Some(libc::ENOTDIR)));
        partial_nondir_dot: resolve("b/c/file/.") => Err(ErrorKind::OsError(Some(libc::ENOTDIR)));
        partial_nondir_dotdot1: resolve("b/c/file/..") => Err(ErrorKind::OsError(Some(libc::ENOTDIR)));
        partial_nondir_dotdot2: resolve("b/c/file/../foo/bar") => Err(ErrorKind::OsError(Some(libc::ENOTDIR)));
        partial_nondir_symlink_slash1: resolve("b-file/") => Err(ErrorKind::OsError(Some(libc::ENOTDIR)));
        partial_nondir_symlink_slash2: resolve("b-file//") => Err(ErrorKind::OsError(Some(libc::ENOTDIR)));
        partial_nondir_symlink_dot: resolve("b-file/.") => Err(ErrorKind::OsError(Some(libc::ENOTDIR)));
        partial_nondir_symlink_dotdot1: resolve("b-file/..") => Err(ErrorKind::OsError(Some(libc::ENOTDIR)));
        partial_nondir_symlink_dotdot2: resolve("b-file/../foo/bar") => Err(ErrorKind::OsError(Some(libc::ENOTDIR)));
        partial_fifo_slash1: resolve("b/fifo/") => Err(ErrorKind::OsError(Some(libc::ENOTDIR)));
        partial_fifo_slash2: resolve("b/fifo//") => Err(ErrorKind::OsError(Some(libc::ENOTDIR)));
        partial_fifo_dot: resolve("b/fifo/.") => Err(ErrorKind::OsError(Some(libc::ENOTDIR)));
        partial_fifo_dotdot1: resolve("b/fifo/..") => Err(ErrorKind::OsError(Some(libc::ENOTDIR)));
        partial_fifo_dotdot2: resolve("b/fifo/../foo/bar") => Err(ErrorKind::OsError(Some(libc::ENOTDIR)));
        partial_sock_slash1: resolve("b/sock/") => Err(ErrorKind::OsError(Some(libc::ENOTDIR)));
        partial_sock_slash2: resolve("b/sock//") => Err(ErrorKind::OsError(Some(libc::ENOTDIR)));
        partial_sock_dot: resolve("b/sock/.") => Err(ErrorKind::OsError(Some(libc::ENOTDIR)));
        partial_sock_dotdot1: resolve("b/sock/..") => Err(ErrorKind::OsError(Some(libc::ENOTDIR)));
        partial_sock_dotdot2: resolve("b/sock/../foo/bar") => Err(ErrorKind::OsError(Some(libc::ENOTDIR)));
        // O_NOFOLLOW doesn't matter for trailing-slash paths.
        partial_symlink_nofollow_slash1: resolve("link3/target_abs/", no_follow_trailing = true) => Ok(("/target", libc::S_IFDIR));
        partial_symlink_nofollow_slash2: resolve("link3/target_abs//", no_follow_trailing = true) => Ok(("/target", libc::S_IFDIR));
        partial_symlink_nofollow_dot: resolve("link3/target_abs/.", no_follow_trailing = true) => Ok(("/target", libc::S_IFDIR));
        // Dangling symlinks are treated as though they are non_existent.
        dangling1_inroot_trailing: resolve("a-fake1") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        dangling1_inroot_partial: resolve("a-fake1/foo") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        dangling1_inroot_partial_dotdot: resolve("a-fake1/../bar/baz") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        dangling1_sub_trailing: resolve("c/a-fake1") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        dangling1_sub_partial: resolve("c/a-fake1/foo") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        dangling1_sub_partial_dotdot: resolve("c/a-fake1/../bar/baz") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        dangling2_inroot_trailing: resolve("a-fake2") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        dangling2_inroot_partial: resolve("a-fake2/foo") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        dangling2_inroot_partial_dotdot: resolve("a-fake2/../bar/baz") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        dangling2_sub_trailing: resolve("c/a-fake2") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        dangling2_sub_partial: resolve("c/a-fake2/foo") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        dangling2_sub_partial_dotdot: resolve("c/a-fake2/../bar/baz") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        dangling3_inroot_trailing: resolve("a-fake3") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        dangling3_inroot_partial: resolve("a-fake3/foo") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        dangling3_inroot_partial_dotdot: resolve("a-fake3/../bar/baz") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        dangling3_sub_trailing: resolve("c/a-fake3") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        dangling3_sub_partial: resolve("c/a-fake3/foo") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        dangling3_sub_partial_dotdot: resolve("c/a-fake3/../bar/baz") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        // Tricky dangling symlinks.
        dangling_tricky1_trailing: resolve("link3/deep_dangling1") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        dangling_tricky1_partial: resolve("link3/deep_dangling1/foo") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        dangling_tricky1_partial_dotdot: resolve("link3/deep_dangling1/..") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        dangling_tricky2_trailing: resolve("link3/deep_dangling2") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        dangling_tricky2_partial: resolve("link3/deep_dangling2/foo") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        dangling_tricky2_partial_dotdot: resolve("link3/deep_dangling2/..") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        // Really deep dangling links.
        deep_dangling1: resolve("dangling/a") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        deep_dangling2: resolve("dangling/b/c") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        deep_dangling3: resolve("dangling/c") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        deep_dangling4: resolve("dangling/d/e") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        deep_dangling5: resolve("dangling/e") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        deep_dangling6: resolve("dangling/g") => Err(ErrorKind::OsError(Some(libc::ENOENT)));
        deep_dangling_fileasdir1: resolve("dangling-file/a") => Err(ErrorKind::OsError(Some(libc::ENOTDIR)));
        deep_dangling_fileasdir2: resolve("dangling-file/b/c") => Err(ErrorKind::OsError(Some(libc::ENOTDIR)));
        deep_dangling_fileasdir3: resolve("dangling-file/c") => Err(ErrorKind::OsError(Some(libc::ENOTDIR)));
        deep_dangling_fileasdir4: resolve("dangling-file/d/e") => Err(ErrorKind::OsError(Some(libc::ENOTDIR)));
        deep_dangling_fileasdir5: resolve("dangling-file/e") => Err(ErrorKind::OsError(Some(libc::ENOTDIR)));
        deep_dangling_fileasdir6: resolve("dangling-file/g") => Err(ErrorKind::OsError(Some(libc::ENOTDIR)));
        // Symlink loops.
        loop1: resolve("loop/link") => Err(ErrorKind::OsError(Some(libc::ELOOP)));
        loop_basic1: resolve("loop/basic-loop1") => Err(ErrorKind::OsError(Some(libc::ELOOP)));
        loop_basic2: resolve("loop/basic-loop2") => Err(ErrorKind::OsError(Some(libc::ELOOP)));
        loop_basic3: resolve("loop/basic-loop3") => Err(ErrorKind::OsError(Some(libc::ELOOP)));
        // NO_FOLLOW.
        symlink_nofollow: resolve("link3/target_abs", no_follow_trailing = true) => Ok(("link3/target_abs", libc::S_IFLNK));
        symlink_component_nofollow1: resolve("e/f", no_follow_trailing = true) => Ok(("b/c/d/e/f", libc::S_IFDIR));
        symlink_component_nofollow2: resolve("link2/link1_abs/target_rel", no_follow_trailing = true) => Ok(("link1/target_rel", libc::S_IFLNK));
        loop_nofollow: resolve("loop/link", no_follow_trailing = true) => Ok(("loop/link", libc::S_IFLNK));
        // RESOLVE_NO_SYMLINKS.
        dir_nosym: resolve("b/c/d/e", rflags = NO_SYMLINKS) => Ok(("b/c/d/e", libc::S_IFDIR));
        symlink_nosym: resolve("link3/target_abs", rflags = NO_SYMLINKS) => Err(ErrorKind::OsError(Some(libc::ELOOP)));
        symlink_component_nosym1: resolve("e/f", rflags = NO_SYMLINKS) => Err(ErrorKind::OsError(Some(libc::ELOOP)));
        symlink_component_nosym2: resolve("link2/link1_abs/target_rel", rflags = NO_SYMLINKS) => Err(ErrorKind::OsError(Some(libc::ELOOP)));
        loop_nosym: resolve("loop/link", rflags = NO_SYMLINKS) => Err(ErrorKind::OsError(Some(libc::ELOOP)));
        // fs.protected_symlinks for a directory owned by us.
        protected_symlinks_selfdir_selfsym: resolve("tmpfs-self/link-self") => Ok(("tmpfs-self/file", libc::S_IFREG));
        protected_symlinks_selfdir_selfsym_nofollow: resolve("tmpfs-self/link-self", no_follow_trailing = true) => Ok(("tmpfs-self/link-self", libc::S_IFLNK));
        #[cfg(feature = "_test_as_root")]
        protected_symlinks_selfdir_otheruidsym: resolve("tmpfs-self/link-otheruid") => Err(ErrorKind::OsError(Some(libc::EACCES)));
        #[cfg(feature = "_test_as_root")]
        protected_symlinks_selfdir_otheruidsym_nofollow: resolve("tmpfs-self/link-otheruid", no_follow_trailing = true) => Ok(("tmpfs-self/link-otheruid", libc::S_IFLNK));
        #[cfg(feature = "_test_as_root")]
        protected_symlinks_selfdir_othergidsym: resolve("tmpfs-self/link-othergid") => Ok(("tmpfs-self/file", libc::S_IFREG));
        #[cfg(feature = "_test_as_root")]
        protected_symlinks_selfdir_othergidsym_nofollow: resolve("tmpfs-self/link-othergid", no_follow_trailing = true) => Ok(("tmpfs-self/link-othergid", libc::S_IFLNK));
        #[cfg(feature = "_test_as_root")]
        protected_symlinks_selfdir_othersym: resolve("tmpfs-self/link-other") => Err(ErrorKind::OsError(Some(libc::EACCES)));
        #[cfg(feature = "_test_as_root")]
        protected_symlinks_selfdir_othersym_nofollow: resolve("tmpfs-self/link-other", no_follow_trailing = true) => Ok(("tmpfs-self/link-other", libc::S_IFLNK));
        // fs.protected_symlinks for a directory owned by someone else.
        #[cfg(feature = "_test_as_root")]
        protected_symlinks_otherdir_selfsym: resolve("tmpfs-other/link-self") => Ok(("tmpfs-other/file", libc::S_IFREG));
        #[cfg(feature = "_test_as_root")]
        protected_symlinks_otherdir_selfsym_nofollow: resolve("tmpfs-other/link-self", no_follow_trailing = true) => Ok(("tmpfs-other/link-self", libc::S_IFLNK));
        #[cfg(feature = "_test_as_root")]
        protected_symlinks_otherdir_selfuidsym: resolve("tmpfs-other/link-selfuid") => Ok(("tmpfs-other/file", libc::S_IFREG));
        #[cfg(feature = "_test_as_root")]
        protected_symlinks_otherdir_selfuidsym_nofollow: resolve("tmpfs-other/link-selfuid", no_follow_trailing = true) => Ok(("tmpfs-other/link-selfuid", libc::S_IFLNK));
        #[cfg(feature = "_test_as_root")]
        protected_symlinks_otherdir_ownersym: resolve("tmpfs-other/link-owner") => Ok(("tmpfs-other/file", libc::S_IFREG));
        #[cfg(feature = "_test_as_root")]
        protected_symlinks_otherdir_ownersym_nofollow: resolve("tmpfs-other/link-owner", no_follow_trailing = true) => Ok(("tmpfs-other/link-owner", libc::S_IFLNK));
        #[cfg(feature = "_test_as_root")]
        protected_symlinks_otherdir_otheruidsym: resolve("tmpfs-other/link-otheruid") => Err(ErrorKind::OsError(Some(libc::EACCES)));
        #[cfg(feature = "_test_as_root")]
        protected_symlinks_otherdir_otheruidsym_nofollow: resolve("tmpfs-other/link-otheruid", no_follow_trailing = true) => Ok(("tmpfs-other/link-otheruid", libc::S_IFLNK));
        #[cfg(feature = "_test_as_root")]
        protected_symlinks_otherdir_othergidsym: resolve("tmpfs-other/link-othergid") => Ok(("tmpfs-other/file", libc::S_IFREG));
        #[cfg(feature = "_test_as_root")]
        protected_symlinks_otherdir_othergidsym_nofollow: resolve("tmpfs-other/link-othergid", no_follow_trailing = true) => Ok(("tmpfs-other/link-othergid", libc::S_IFLNK));
        #[cfg(feature = "_test_as_root")]
        protected_symlinks_otherdir_othersym: resolve("tmpfs-other/link-other") => Err(ErrorKind::OsError(Some(libc::EACCES)));
        #[cfg(feature = "_test_as_root")]
        protected_symlinks_otherdir_othersym_nofollow: resolve("tmpfs-other/link-other", no_follow_trailing = true) => Ok(("tmpfs-other/link-other", libc::S_IFLNK));
    }
}

resolve_tests! {
    [with_nosymfollow_root] {
        #[cfg(feature = "_test_as_root")]
        symlink_regular_symfollow: resolve("nosymfollow/goodlink") => Ok((".", libc::S_IFDIR));
        #[cfg(feature = "_test_as_root")]
        symlink_itself_nosymfollow: resolve("nosymfollow/badlink") => Err(ErrorKind::OsError(Some(libc::ELOOP)));
        #[cfg(feature = "_test_as_root")]
        symlink_inside_nosymfollow_dir: resolve("nosymfollow/nosymdir/dir/badlink") => Err(ErrorKind::OsError(Some(libc::ELOOP)));
        #[cfg(feature = "_test_as_root")]
        symlink_itself_nested_clear_symfollow: resolve("nosymfollow/nosymdir/dir/goodlink") => Ok((".", libc::S_IFDIR));
        #[cfg(feature = "_test_as_root")]
        symlink_inside_nested_clear_symfollow: resolve("nosymfollow/nosymdir/dir/foo/yessymdir/bar/goodlink") => Ok((".", libc::S_IFDIR));
    }
}

mod utils {
    use crate::{
        error::ErrorKind,
        flags::OpenFlags,
        syscalls,
        tests::{
            common::{self as tests_common, LookupResult},
            traits::{HandleImpl, RootImpl},
        },
        utils::FdExt,
    };

    use std::{os::unix::fs::MetadataExt, path::Path};

    use anyhow::{Context, Error};
    use pretty_assertions::assert_eq;

    pub(super) fn with_basic_tree<F>(func: F) -> Result<(), Error>
    where
        F: FnOnce(&Path) -> Result<(), Error>,
    {
        let root_dir = tests_common::create_basic_tree()?;

        let res = func(root_dir.path());

        let _root_dir = root_dir; // make sure tmpdir is kept alive
        res
    }

    pub(super) fn with_proc<F>(func: F) -> Result<(), Error>
    where
        F: FnOnce(&Path) -> Result<(), Error>,
    {
        func(Path::new("/proc"))
    }

    pub(super) fn with_nosymfollow_root<F>(func: F) -> Result<(), Error>
    where
        F: FnOnce(&Path) -> Result<(), Error>,
    {
        tests_common::in_mnt_ns(|| {
            let root_dir = tests_common::create_basic_tree()?;

            tests_common::mask_nosymfollow(root_dir.path())?;

            let res = func(root_dir.path());

            let _root_dir = root_dir; // make sure tmpdir is kept alive
            res
        })
    }

    pub(super) fn check_root_resolve<R, H>(
        root: R,
        unsafe_path: impl AsRef<Path>,
        no_follow_trailing: bool,
        expected: Result<LookupResult, ErrorKind>,
    ) -> Result<(), Error>
    where
        H: HandleImpl,
        R: RootImpl<Handle = H>,
        for<'a> &'a R::Handle: HandleImpl,
    {
        let root_dir = root.as_unsafe_path_unchecked()?;
        let unsafe_path = unsafe_path.as_ref();

        let result = if no_follow_trailing {
            root.resolve_nofollow(unsafe_path)
        } else {
            root.resolve(unsafe_path)
        };

        let (handle, expected_path, expected_file_type) = match (result, expected) {
            (Ok(handle), Ok((expected_path, file_type))) => (
                handle,
                expected_path.replace("{{/proc/self}}", &syscalls::getpid().to_string()),
                file_type,
            ),
            (result, expected) => {
                let result = match result {
                    Ok(handle) => Ok(handle.as_unsafe_path_unchecked()?),
                    Err(err) => Err(err),
                };

                tests_common::check_err(&result, &expected)
                    .with_context(|| format!("root resolve {unsafe_path:?}"))?;

                assert!(
                    result.is_err(),
                    "we should never see an Ok(file) after check_err if we expected {expected:?}"
                );
                return Ok(());
            }
        };

        let expected_path = expected_path.trim_start_matches('/');
        let real_handle_path = handle.as_unsafe_path_unchecked()?;
        assert_eq!(
            real_handle_path,
            root_dir.join(expected_path),
            "resolve({unsafe_path:?}, {no_follow_trailing}) path mismatch",
        );

        let meta = handle.metadata()?;
        let real_file_type = meta.mode() & libc::S_IFMT;
        assert_eq!(real_file_type, expected_file_type, "file type mismatch",);

        match real_file_type {
            libc::S_IFDIR => {
                tests_common::check_reopen(&handle, OpenFlags::O_RDONLY, None)?;
                tests_common::check_reopen(&handle, OpenFlags::O_DIRECTORY, None)?;
                // Make sure O_NOFOLLOW acts the way you would expect.
                tests_common::check_reopen(
                    &handle,
                    OpenFlags::O_RDONLY | OpenFlags::O_NOFOLLOW,
                    None,
                )?;
                tests_common::check_reopen(
                    &handle,
                    OpenFlags::O_PATH | OpenFlags::O_NOFOLLOW,
                    None,
                )?;
                tests_common::check_reopen(&handle, OpenFlags::O_PATH, None)?;
                // Forcefully set O_CLOEXEC.
                tests_common::check_reopen(
                    &handle,
                    OpenFlags::O_RDONLY | OpenFlags::O_CLOEXEC,
                    None,
                )?;
                tests_common::check_reopen(
                    &handle,
                    OpenFlags::O_DIRECTORY | OpenFlags::O_CLOEXEC,
                    None,
                )?;
            }
            libc::S_IFREG => {
                tests_common::check_reopen(&handle, OpenFlags::O_RDWR, None)?;
                tests_common::check_reopen(&handle, OpenFlags::O_DIRECTORY, Some(libc::ENOTDIR))?;
                // Make sure O_NOFOLLOW acts the way you would expect.
                tests_common::check_reopen(
                    &handle,
                    OpenFlags::O_RDONLY | OpenFlags::O_NOFOLLOW,
                    None,
                )?;
                tests_common::check_reopen(
                    &handle,
                    OpenFlags::O_PATH | OpenFlags::O_NOFOLLOW,
                    None,
                )?;
                tests_common::check_reopen(&handle, OpenFlags::O_PATH, None)?;
                // Forcefully set O_CLOEXEC.
                tests_common::check_reopen(
                    &handle,
                    OpenFlags::O_RDWR | OpenFlags::O_CLOEXEC,
                    None,
                )?;
                tests_common::check_reopen(
                    &handle,
                    OpenFlags::O_DIRECTORY | OpenFlags::O_CLOEXEC,
                    Some(libc::ENOTDIR),
                )?;
            }
            libc::S_IFLNK => {
                assert!(
                    no_follow_trailing,
                    "we must only get a symlink handle if no_follow_trailing is set"
                );

                tests_common::check_reopen(&handle, OpenFlags::O_RDONLY, Some(libc::ELOOP))
                    .context("reopen(O_RDONLY) of a symlink handle should fail with ELOOP")?;
                tests_common::check_reopen(&handle, OpenFlags::O_PATH, Some(libc::ELOOP))
                    .context("reopen(O_PATH) of a symlink handle should fail with ELOOP")?;
                tests_common::check_reopen(
                    &handle,
                    OpenFlags::O_PATH | OpenFlags::O_NOFOLLOW,
                    Some(libc::ELOOP),
                )
                .context("reopen(O_PATH|O_NOFOLLOW) of a symlink handle should fail with ELOOP")?;
            }
            _ => {
                tests_common::check_reopen(&handle, OpenFlags::O_PATH, None)?;
                tests_common::check_reopen(
                    &handle,
                    OpenFlags::O_PATH | OpenFlags::O_DIRECTORY,
                    Some(libc::ENOTDIR),
                )?;
                // Make sure O_NOFOLLOW acts the way you would expect.
                tests_common::check_reopen(
                    &handle,
                    OpenFlags::O_PATH | OpenFlags::O_NOFOLLOW,
                    None,
                )?;
                // Forcefully set O_CLOEXEC.
                tests_common::check_reopen(
                    &handle,
                    OpenFlags::O_PATH | OpenFlags::O_CLOEXEC,
                    None,
                )?;
                tests_common::check_reopen(
                    &handle,
                    OpenFlags::O_PATH | OpenFlags::O_DIRECTORY | OpenFlags::O_CLOEXEC,
                    Some(libc::ENOTDIR),
                )?;
            }
        }

        Ok(())
    }
}
