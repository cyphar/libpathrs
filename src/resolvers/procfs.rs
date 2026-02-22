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

//!
//! [`ProcfsResolver`](crate::resolvers::procfs::ProcfsResolver) is a very
//! minimal resolver that doesn't allow:
//!
//!  1. Any ".." components (with `openat2` this is slightly relaxed).
//!  2. Any absolute symlinks.
//!  3. (If `statx` or `openat2` is supported), any mount-point crossings.
//!
//! This allows us to avoid using any `/proc` checks, and thus this resolver can
//! be used within the `pathrs::procfs` helpers that are used by other parts of
//! libpathrs.

use crate::{
    error::{Error, ErrorExt, ErrorImpl},
    flags::{OpenFlags, ResolverFlags},
    procfs,
    resolvers::MAX_SYMLINK_TRAVERSALS,
    syscalls::{self, OpenHow},
    utils::{self, FdExt, PathIterExt, RawProcfsRoot},
};

use std::{
    collections::VecDeque,
    io::Error as IOError,
    os::unix::{
        ffi::OsStrExt,
        io::{AsFd, OwnedFd},
    },
    path::Path,
};

/// Used internally for tests to force the usage of a specific resolver. You
/// should always use the default.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ProcfsResolver {
    Openat2,
    RestrictedOpath,
}

impl Default for ProcfsResolver {
    fn default() -> Self {
        // Only check if there is a cached failure from a previous attempt to
        // use openat2 -- we don't want to do a dummy openat2(2) call here in
        // Default, since it gets called a lot by C FFI. If openat2(2) is
        // unsupported, we will detect it later.
        if syscalls::openat2::saw_openat2_failure() {
            Self::RestrictedOpath
        } else {
            Self::Openat2
        }
    }
}

impl ProcfsResolver {
    pub(crate) fn resolve(
        &self,
        proc_rootfd: RawProcfsRoot<'_>,
        root: impl AsFd,
        path: impl AsRef<Path>,
        oflags: OpenFlags,
        rflags: ResolverFlags,
    ) -> Result<OwnedFd, Error> {
        // These flags don't make sense for procfs and will just result in
        // confusing errors during lookup. O_TMPFILE contains multiple flags
        // (including O_DIRECTORY!) so we have to check it separately.
        let invalid_flags = OpenFlags::O_CREAT | OpenFlags::O_EXCL;
        if !oflags.intersection(invalid_flags).is_empty() || oflags.contains(OpenFlags::O_TMPFILE) {
            Err(ErrorImpl::InvalidArgument {
                name: "flags".into(),
                description: format!(
                    "invalid flags {:?} specified",
                    oflags.intersection(invalid_flags)
                )
                .into(),
            })?
        }

        let root = root.as_fd();
        let path = path.as_ref();

        match *self {
            Self::Openat2 => openat2_resolve(root, path, oflags, rflags).or_else(|err| {
                // If an error occurred, it could be due to openat2(2) being
                // disabled via seccomp or just being unsupported. We check this
                // via a dummy openat2(2) chall -- if that fails then we
                // fallback to O_PATH, otherwise we assume openat2(2) failed for
                // a good reason and return that error outright.
                //
                // TODO: Find a way to make this fallback logic a bit less
                //       repetitive of the other match arm.
                if syscalls::openat2::openat2_is_not_supported() {
                    opath_resolve(proc_rootfd, root, path, oflags, rflags)
                } else {
                    Err(err)
                }
            }),
            Self::RestrictedOpath => opath_resolve(proc_rootfd, root, path, oflags, rflags),
        }
    }
}

/// [`openat2`][openat2.2]-based implementation of [`ProcfsResolver`].
///
/// [openat2.2]: https://www.man7.org/linux/man-pages/man2/openat2.2.html
fn openat2_resolve(
    root: impl AsFd,
    path: impl AsRef<Path>,
    oflags: OpenFlags,
    rflags: ResolverFlags,
) -> Result<OwnedFd, Error> {
    // Copy the O_NOFOLLOW and RESOLVE_NO_SYMLINKS bits from rflags.
    let oflags = oflags.bits() as u64;
    let rflags =
        libc::RESOLVE_BENEATH | libc::RESOLVE_NO_MAGICLINKS | libc::RESOLVE_NO_XDEV | rflags.bits();

    syscalls::openat2_follow(
        root,
        path,
        OpenHow {
            flags: oflags,
            resolve: rflags,
            ..Default::default()
        },
    )
    .map_err(|err| {
        ErrorImpl::RawOsError {
            operation: "open subpath in procfs".into(),
            source: err,
        }
        .into()
    })
}

/// Returns whether the provided string plausibly looks like a magic-link
/// `readlink(2)` target.
fn check_possible_magic_link(link_target: &Path) -> Result<(), Error> {
    // This resolver only deals with procfs paths, which means that we can
    // restrict how we handle symlinks. procfs does not (and cannot) contain
    // regular absolute symlinks to paths within procfs, and so we can assume
    // any absolute paths are magic-links to regular files or would otherwise
    // trigger EXDEV with openat2. (Note that all procfs magic-links use
    // `d_path` as the readlink(2) pseudo-target.)
    if link_target.is_absolute() {
        Err(ErrorImpl::OsError {
            operation: "emulated RESOLVE_NO_MAGICLINKS".into(),
            source: IOError::from_raw_os_error(libc::ELOOP),
        })
        .wrap(format!("step into absolute symlink {link_target:?}"))?
    }

    // However, some magic-links appear as relative paths because they reference
    // custom anon-inodes or other objects with custom `d_path` callbacks (and
    // thus custom names). Without openat2(2) there isn't an obvious way to
    // detect this with 100% accuracy, but we can safely assume that no regular
    // symlink will have names that look like these special symlinks (they
    // typically look like "foo:[bar]").
    //
    // For reference, at time of writing (Linux 6.17), all of the regular
    // symlinks in stock procfs (and their corresponding readlink targets)
    // are listed below.
    //
    //  * /proc/self -> "<pid>" (auto-generated)
    //  * /proc/thread-self -> "<pid>/task/<tid>" (auto-generated)
    //  * /proc/net -> "self/net"
    //  * /proc/mounts -> "self/mounts"
    //
    // Followed by the following procfs symlinks defined by other modules
    // (using proc_symlink()):
    //
    //  * /proc/ppc64 -> "powerpc" (on ppc64)
    //  * /proc/rtas -> "powerpc/rtas" (on ppc)
    //  * /proc/device-tree -> "/sys/firmware/devicetree/base"
    //  * /proc/fs/afs -> "../self/net/afs" (afs)
    //  * /proc/fs/fscache -> "netfs" (netfs)
    //  * /proc/fs/nfsfs/servers -> "../../net/nfsfs/servers" (nfs)
    //  * /proc/fs/nfsfs/volumes -> "../../net/nfsfs/volumes" (nfs)
    //  * /proc/fs/xfs/stat -> "/sys/fs/xfs/stat/stats" (xfs)
    //  * /proc/asound/<id> -> "<card-name>" (sound)
    //
    // As you can see, none of them match the format of anon-inodes and so
    // blocking symlinks that look like that is reasonable. It is possible for
    // /proc/asound/* symlinks to have arbitrary data, but it seems very
    // unlikely for a card to have a name that looks like "foo:[bar]".

    // The regex crate is too heavy for us to use it for such a simple string
    // match. Instead, let's just do a quick-and-dirty search to see if the
    // characters ":[]" are present in the string and are in the right order.
    // MSRV(1.65): Switch to regex-lite?
    if link_target
        .as_os_str()
        .to_string_lossy()
        .chars()
        .filter(|&c| c == ':' || c == '[' || c == ']')
        .collect::<String>()
        == ":[]"
    {
        Err(ErrorImpl::OsError {
            operation: "emulated RESOLVE_NO_MAGICLINKS".into(),
            source: IOError::from_raw_os_error(libc::ELOOP),
        })
        .wrap(format!("step into likely magiclink {link_target:?}"))?
    }

    Ok(())
}

/// `O_PATH`-based implementation of [`ProcfsResolver`].
fn opath_resolve(
    proc_rootfd: RawProcfsRoot<'_>,
    root: impl AsFd,
    path: impl AsRef<Path>,
    oflags: OpenFlags,
    rflags: ResolverFlags,
) -> Result<OwnedFd, Error> {
    let root = root.as_fd();
    let root_mnt_id = utils::fetch_mnt_id(proc_rootfd, root, "")?;

    // We only need to keep track of our current dirfd, since we are applying
    // the components one-by-one.
    let mut current = root
        .try_clone_to_owned()
        .map_err(|err| ErrorImpl::OsError {
            operation: "dup root handle as starting point of resolution".into(),
            source: err,
        })?;

    // In order to match the behaviour of RESOLVE_BENEATH, we need to error out
    // if we get asked to resolve an absolute path.
    let path = path.as_ref();
    if path.is_absolute() {
        Err(ErrorImpl::OsError {
            operation: "emulated RESOLVE_BENEATH".into(),
            source: IOError::from_raw_os_error(libc::EXDEV),
        })
        .wrap(format!(
            "requested subpath {path:?} is absolute but this is forbidden by RESOLVE_BENEATH",
        ))?
    }

    // Get initial set of components from the passed path. We remove components
    // as we do the path walk, and update them with the contents of any symlinks
    // we encounter. Path walking terminates when there are no components left.
    let mut remaining_components = path
        .raw_components()
        .map(|p| p.to_os_string())
        .collect::<VecDeque<_>>();

    let mut symlink_traversals = 0;
    while let Some(part) = remaining_components
        .pop_front()
        // If we hit an empty component, we need to treat it as though it is
        // "." so that trailing "/" and "//" components on a non-directory
        // correctly return the right error code.
        .map(|part| if part.is_empty() { ".".into() } else { part })
    {
        // We cannot walk into ".." without checking if there was a breakout
        // with /proc (a-la opath::resolve) so return an error if we hit "..".
        if part.as_bytes() == b".." {
            Err(ErrorImpl::OsError {
                operation: "step into '..'".into(),
                source: IOError::from_raw_os_error(libc::EXDEV),
            })
            .wrap("cannot walk into '..' with restricted procfs resolver")?
        }

        // Get our next element.
        let next = syscalls::openat(
            &current,
            &part,
            OpenFlags::O_PATH | OpenFlags::O_NOFOLLOW,
            0,
        )
        .map_err(|err| ErrorImpl::RawOsError {
            operation: "open next component of resolution".into(),
            source: err,
        })?;

        // Check that the next component is on the same mountpoint.
        // NOTE: If the root is the host /proc mount, this is only safe if there
        // are no racing mounts.
        procfs::verify_same_procfs_mnt(proc_rootfd, root_mnt_id, &next)
            .with_wrap(|| format!("open next component {part:?}"))
            .wrap("emulated procfs resolver RESOLVE_NO_XDEV")?;

        let next_meta = next.metadata().wrap("fstat of next component")?;

        // If this is the last component, try to open the same component again
        // with with the requested flags. Unlike the other Handle resolvers, we
        // can't re-open the file through procfs (since this is the resolver
        // used for procfs lookups) so we need to do it this way.
        //
        // Because we force O_NOFOLLOW for safety reasons, we can't just blindly
        // return the error we get from openat here (in particular, if the user
        // specifies O_PATH or O_DIRECTORY without O_NOFOLLOW, you will get the
        // wrong results). The following is a table of the relevant cases.
        //
        // Each entry of the form [a](b) means that the user expects [a] to
        // happen but because of O_NOFOLLOW we get (b). **These are the cases
        // which we need to handle with care.**
        //
        //                   symlink        directory    other-file
        //
        // OPATH         [cont](ret-sym) *1    ret           ret
        // ODIR          [cont](ENOTDIR) *2    ret         ENOTDIR
        // OPATH|ODIR    [cont](ENOTDIR) *3    ret         ENOTDIR
        // ONF               ELOOP             ret           ret
        // ONF|OPATH        ret-sym      *4    ret           ret
        // ONF|ODIR         ENOTDIR            ret         ENOTDIR
        // ONF|OPATH|ODIR   ENOTDIR            ret         EDOTDIR
        //
        // Legend:
        // - Flags:
        //   - OPATH = O_PATH, ODIR = O_DIRECTORY, ONF = O_NOFOLLOW
        // - Actions:
        //   - ret     = return this handle as the final component
        //   - ret-sym = return this *symlink* handle as the final component
        //   - cont    = continue iterating (for symlinks)
        //   - EFOO    = returns an error EFOO
        //
        // Unfortunately, note that you -ENOTDIR for most of the file and
        // symlink cases, but we need to differentiate between them. That's why
        // we need to do the O_PATH|O_NOFOLLOW first -- we need to figure out
        // whether we are dealing with a symlink or not. If we are dealing with
        // a symlink, we want to continue walking in all cases (except plain
        // O_NOFOLLOW and O_DIRECTORY|O_NOFOLLOW).
        //
        // NOTE: There is a possible race here -- the file type might've changed
        // after we opened it. This is unlikely under procfs because the
        // structure is basically static (an attacker could bind-mount something
        // but we detect bind-mounts already), but even if it did happen the
        // worst case result is that we return an error.
        //
        // NOTE: Most of these cases don't apply to the ProcfsResolver because
        // it handles trailing-symlink follows manually and auto-applies
        // O_NOFOLLOW if the trailing component is not a symlink. However, we
        // handle them all for correctness reasons (and we have tests for the
        // resolver itself to verify the behaviour).
        if remaining_components.is_empty()
            // Case (*1):
            // If the user specified *just* O_PATH (without O_NOFOLLOW nor
            // O_DIRECTORY), we can continue to parse as normal (if next_type is
            // a non-symlink we will return it, if it is a symlink we will
            // continue walking).
            && oflags.intersection(OpenFlags::O_PATH | OpenFlags::O_NOFOLLOW | OpenFlags::O_DIRECTORY) != OpenFlags::O_PATH
        {
            match syscalls::openat(&current, &part, oflags | OpenFlags::O_NOFOLLOW, 0) {
                // NOTE: This will silently add O_NOFOLLOW to the set of flags
                // you see in fcntl(F_GETFL). In practice this isn't an issue,
                // but it is a detectable difference between the O_PATH resolver
                // and openat2. Unfortunately F_SETFL silently ignores
                // O_NOFOLLOW so we cannot clear this flag (the only option
                // would be a procfs re-open -- but this *is* the procfs re-open
                // code!).
                Ok(final_reopen) => {
                    // Re-verify the next component is on the same mount.
                    procfs::verify_same_procfs_mnt(proc_rootfd, root_mnt_id, &final_reopen)
                        .wrap("re-open final component")
                        .wrap("emulated procfs resolver RESOLVE_NO_XDEV")?;
                    return Ok(final_reopen);
                }
                Err(err) => {
                    // Cases (*2) and (*3):
                    //
                    // If all of the following are true:
                    //
                    //  1. The user didn't ask for O_NOFOLLOW.
                    //  2. The user did ask for O_DIRECTORY.
                    //  3. The error is ENOTDIR.
                    //  4. The next component was a symlink.
                    //
                    // We want to continue walking, rather than return an error.
                    if oflags.contains(OpenFlags::O_NOFOLLOW)
                        || !oflags.contains(OpenFlags::O_DIRECTORY)
                        || err.root_cause().raw_os_error() != Some(libc::ENOTDIR)
                        || !next_meta.is_symlink()
                    {
                        Err(ErrorImpl::RawOsError {
                            operation: format!("open last component of resolution with {oflags:?}")
                                .into(),
                            source: err,
                        })?
                    }
                }
            }
        }

        // Is the next dirfd a symlink or an ordinary path? If we're an ordinary
        // dirent, we just update current and move on to the next component.
        // Nothing special here.
        if !next_meta.is_symlink() {
            current = next;
            continue;
        }

        // Don't continue walking if user asked for no symlinks.
        if rflags.contains(ResolverFlags::NO_SYMLINKS) {
            Err(ErrorImpl::OsError {
                operation: "emulated symlink resolution".into(),
                source: IOError::from_raw_os_error(libc::ELOOP),
            })
            .wrap(format!(
                "component {part:?} is a symlink but symlink resolution is disabled",
            ))?
        }

        // We need a limit on the number of symlinks we traverse to avoid
        // hitting filesystem loops and DoSing.
        //
        // Given all of the other restrictions of this lookup code, it seems
        // unlikely that you could even run into a symlink loop (procfs doesn't
        // have regular symlink loops) but we should avoid it just in case.
        symlink_traversals += 1;
        if symlink_traversals >= MAX_SYMLINK_TRAVERSALS {
            Err(ErrorImpl::OsError {
                operation: "emulated symlink resolution".into(),
                source: IOError::from_raw_os_error(libc::ELOOP),
            })
            .wrap("exceeded symlink limit")?
        }

        let link_target = syscalls::readlinkat(&next, "").map_err(|err| ErrorImpl::RawOsError {
            operation: "readlink next symlink component".into(),
            source: err,
        })?;

        // If this symlink is a magic-link, we will likely end up trying to walk
        // into a non-existent path (or possibly an attacker-controlled procfs
        // subpath) so we reject any link target that looks like a magic-link.
        check_possible_magic_link(&link_target)
            .wrap("cannot walk into potential magiclinks with restricted procfs resolver")?;

        link_target
            .raw_components()
            .prepend(&mut remaining_components);
    }

    Ok(current)
}

#[cfg(test)]
mod tests {
    use crate::{
        error::{Error as PathrsError, ErrorKind},
        flags::{OpenFlags, ResolverFlags},
        resolvers::procfs::ProcfsResolver,
        syscalls,
        utils::{FdExt, RawProcfsRoot},
    };

    use std::{
        fs::File,
        os::unix::io::{AsRawFd, OwnedFd, RawFd},
        path::{Path, PathBuf},
    };

    use anyhow::{Context, Error};
    use pretty_assertions::{assert_eq, assert_matches};
    use rustix::io as rustix_io;

    type ExpectedResult = Result<PathBuf, ErrorKind>;

    /// Dummy type to create a symlink loop in procfs that fulfils
    /// `AsRef<Path>`.
    #[derive(Debug)]
    struct FdSymlinkLoop {
        fd: OwnedFd,
        fd_subpath: PathBuf,
    }

    impl AsRef<Path> for FdSymlinkLoop {
        fn as_ref(&self) -> &Path {
            &self.fd_subpath
        }
    }

    impl AsRawFd for FdSymlinkLoop {
        fn as_raw_fd(&self) -> RawFd {
            self.fd.as_raw_fd()
        }
    }

    impl FdSymlinkLoop {
        fn new() -> Result<Self, Error> {
            // In order to create a symlink loop in procfs, we create a dummy
            // file with fd $n, open /proc/self/fd/$n as O_PATH|O_NOFOLLOW, and
            // then dup2(2) the second fd over $n.
            let mut target_fd = syscalls::openat(syscalls::AT_FDCWD, ".", OpenFlags::O_RDONLY, 0)
                .context("open dummy fd")?;

            let fdlink_subpath = PathBuf::from(format!("self/fd/{}", target_fd.as_raw_fd()));

            let fdlink_fd = syscalls::openat(
                syscalls::BADFD,
                PathBuf::from("/proc").join(&fdlink_subpath),
                OpenFlags::O_PATH | OpenFlags::O_NOFOLLOW,
                0,
            )
            .context("open proc fdlink")?;

            rustix_io::dup2(fdlink_fd, &mut target_fd)
                .context("dup fdlink handle over the original fd")?;

            Ok(Self {
                fd: target_fd,
                fd_subpath: fdlink_subpath,
            })
        }
    }

    macro_rules! procfs_resolver_tests {
        () => {};

        ($(#[$meta:meta])* $test_name:ident ($root:expr, $path:expr, $($oflag:ident)|+, $rflags:expr) == $expected_result:expr ; $($rest:tt)*) => {
            procfs_resolver_tests! {
                $(#[$meta])*
                $test_name($root, path @ $path, $($oflag)|*, $rflags) == $expected_result;

                $($rest)*
            }
        };

        ($(#[$meta:meta])* $test_name:ident ($root:expr, $path_var:ident @ $path:expr, $($oflag:ident)|+, $rflags:expr) == $expected_result:expr ; $($rest:tt)*) => {
            paste::paste! {
                #[test]
                $(#[$meta])*
                fn [<procfs_openat2_resolver_ $test_name>]() -> Result<(), Error> {
                    // TODO: Drop this?
                    if syscalls::openat2::openat2_is_not_supported() {
                        return Ok(());
                    }
                    let root_dir: PathBuf = $root.into();
                    let root = File::open(&root_dir)?;
                    let $path_var = $path;
                    let expected: ExpectedResult = $expected_result.map(|p: PathBuf| root_dir.join(p));
                    let oflags = $(OpenFlags::$oflag)|*;
                    let res = ProcfsResolver::Openat2
                        .resolve(RawProcfsRoot::UnsafeGlobal, &root, &$path_var, oflags, $rflags)
                        .as_ref()
                        .map(|f| {
                            f.as_unsafe_path_unchecked()
                                .expect("get actual path of resolved handle")
                        })
                        .map_err(PathrsError::kind);
                    assert_eq!(
                        res, expected,
                        "expected resolve({:?}, {:?}, {:?}, {:?}) to give {:?}, got {:?}",
                        $root, &$path_var, oflags, $rflags, expected, res
                    );
                    Ok(())
                }

                #[test]
                $(#[$meta])*
                fn [<procfs_opath_resolver_ $test_name>]() -> Result<(), Error> {
                    let root_dir: PathBuf = $root.into();
                    let root = File::open(&root_dir)?;
                    let $path_var = $path;
                    let expected: ExpectedResult = $expected_result.map(|p: PathBuf| root_dir.join(p));
                    let oflags = $(OpenFlags::$oflag)|*;
                    let res = ProcfsResolver::RestrictedOpath
                        .resolve(RawProcfsRoot::UnsafeGlobal, &root, &$path_var, oflags, $rflags)
                        .as_ref()
                        .map(|f| {
                            f.as_unsafe_path_unchecked()
                                .expect("get actual path of resolved handle")
                        })
                        .map_err(PathrsError::kind);
                    assert_eq!(
                        res, expected,
                        "expected resolve({:?}, {:?}, {:?}, {:?}) to give {:?}, got {:?}",
                        $root, &$path_var, oflags, $rflags, expected, res
                    );
                    Ok(())
                }
            }

            procfs_resolver_tests! { $($rest)* }
        };
    }

    procfs_resolver_tests! {
        xdev("/", "proc", O_DIRECTORY, ResolverFlags::empty()) == Err(ErrorKind::OsError(Some(libc::EXDEV)));
        xdev_dotdot("/proc", "..", O_DIRECTORY, ResolverFlags::empty()) == Err(ErrorKind::OsError(Some(libc::EXDEV)));
        xdev_abs_slash("/proc", "/", O_DIRECTORY, ResolverFlags::empty()) == Err(ErrorKind::OsError(Some(libc::EXDEV)));
        xdev_abs_path("/proc", "/etc/passwd", O_DIRECTORY, ResolverFlags::empty()) == Err(ErrorKind::OsError(Some(libc::EXDEV)));
        bad_flag_ocreat("/tmp", "foobar", O_CREAT|O_RDWR, ResolverFlags::empty()) == Err(ErrorKind::InvalidArgument);
        bad_flag_otmpfile("/tmp", "foobar", O_TMPFILE|O_RDWR, ResolverFlags::empty()) == Err(ErrorKind::InvalidArgument);

        // Check RESOLVE_NO_SYMLINKS handling.
        resolve_no_symlinks1("/proc", "self", O_DIRECTORY, ResolverFlags::NO_SYMLINKS) == Err(ErrorKind::OsError(Some(libc::ELOOP)));
        resolve_no_symlinks2("/proc", "self/status", O_RDONLY, ResolverFlags::NO_SYMLINKS) == Err(ErrorKind::OsError(Some(libc::ELOOP)));
        resolve_no_symlinks3("/proc", "self/../cgroups", O_RDONLY, ResolverFlags::NO_SYMLINKS) == Err(ErrorKind::OsError(Some(libc::ELOOP)));

        // Check RESOLVE_NO_MAGICLINKS handling.
        symlink("/proc", "self", O_DIRECTORY, ResolverFlags::empty()) == Ok(format!("/proc/{}", syscalls::getpid()).into());
        symlink_onofollow("/proc", "mounts", O_NOFOLLOW, ResolverFlags::empty()) == Err(ErrorKind::OsError(Some(libc::ELOOP)));
        symlink_opath_onofollow("/proc", "mounts", O_PATH|O_NOFOLLOW, ResolverFlags::empty()) == Ok("mounts".into());
        // These tests don't really work with "cargo test" because in a
        // multi-threaded program the thread-group leader can change after the
        // previous one dies during the test execution, causing the file to be
        // shown as "deleted" (but not consistently because it's a race
        // condition). We could be more lenient here but it would make the
        // checking code more complicated.
        //
        // NOTE: cfg(nextest) is something specific to libpathrs, upstream
        // nextest does not provide a cfg like this. See the discussion in
        // <https://github.com/nextest-rs/nextest/discussions/2789>.
        #[cfg_attr(not(nextest), ignore)]
        symlink_parent("/proc", "net/unix", O_RDONLY, ResolverFlags::empty()) == Ok(format!("/proc/{}/net/unix", syscalls::getpid()).into());
        #[cfg_attr(not(nextest), ignore)]
        symlink_parent_onofollow("/proc", "net/unix", O_NOFOLLOW, ResolverFlags::empty()) == Ok(format!("/proc/{}/net/unix", syscalls::getpid()).into());
        #[cfg_attr(not(nextest), ignore)]
        symlink_parent_opath_onofollow("/proc", "net/unix", O_PATH|O_NOFOLLOW, ResolverFlags::empty()) == Ok(format!("/proc/{}/net/unix", syscalls::getpid()).into());

        magiclink_absolute("/proc", "self/fd/0", O_RDWR, ResolverFlags::empty()) == Err(ErrorKind::OsError(Some(libc::ELOOP)));
        magiclink_absolute_onofollow("/proc", "self/fd/0", O_NOFOLLOW, ResolverFlags::empty()) == Err(ErrorKind::OsError(Some(libc::ELOOP)));
        magiclink_absolute_opath_onofollow("/proc", "self/fd/0", O_PATH|O_NOFOLLOW, ResolverFlags::empty()) == Ok(format!("/proc/{}/fd/0", syscalls::getpid()).into());
        magiclink_absolute_parent("/proc", "self/root/etc/passwd", O_RDONLY, ResolverFlags::empty()) == Err(ErrorKind::OsError(Some(libc::ELOOP)));
        magiclink_absolute_parent_onofollow("/proc", "self/cwd/foo", O_NOFOLLOW, ResolverFlags::empty()) == Err(ErrorKind::OsError(Some(libc::ELOOP)));
        magiclink_absolute_parent_opath_onofollow("/proc", "self/cwd/abc", O_PATH|O_NOFOLLOW, ResolverFlags::empty()) == Err(ErrorKind::OsError(Some(libc::ELOOP)));
        magiclink_anoninode("/proc", "self/ns/pid", O_PATH, ResolverFlags::empty()) == Err(ErrorKind::OsError(Some(libc::ELOOP)));
        magiclink_anoninode_onofollow("/proc", "self/ns/user", O_NOFOLLOW, ResolverFlags::empty()) == Err(ErrorKind::OsError(Some(libc::ELOOP)));
        magiclink_anoninode_opath_nofollow("/proc", "self/ns/user", O_PATH|O_NOFOLLOW, ResolverFlags::empty()) == Ok(format!("/proc/{}/ns/user", syscalls::getpid()).into());
        magiclink_anoninode_parent("/proc", "self/ns/mnt/foo", O_RDONLY, ResolverFlags::empty()) == Err(ErrorKind::OsError(Some(libc::ELOOP)));
        magiclink_anoninode_parent_onofollow("/proc", "self/ns/mnt/foo", O_NOFOLLOW, ResolverFlags::empty()) == Err(ErrorKind::OsError(Some(libc::ELOOP)));
        magiclink_anoninode_parent_opath_onofollow("/proc", "self/ns/uts/foo", O_PATH|O_NOFOLLOW, ResolverFlags::empty()) == Err(ErrorKind::OsError(Some(libc::ELOOP)));

        // Check symlink loops.
        symloop("/proc", FdSymlinkLoop::new()?, O_PATH, ResolverFlags::empty()) == Err(ErrorKind::OsError(Some(libc::ELOOP)));
        symloop_opath_onofollow("/proc", fdpath @ FdSymlinkLoop::new()?, O_PATH|O_NOFOLLOW, ResolverFlags::empty()) == Ok(format!("/proc/{}/fd/{}", syscalls::getpid(), fdpath.as_raw_fd()).into());

        // Check that our {O_PATH, O_NOFOLLOW, O_DIRECTORY} logic is correct,
        // based on the table in opath_resolve().

        // OPATH         [cont](ret)     *1    ret           ret
        sym_opath("/proc", "self", O_PATH, ResolverFlags::empty()) == Ok(format!("/proc/{}", syscalls::getpid()).into());
        dir_opath("/proc", "tty", O_PATH, ResolverFlags::empty()) == Ok("/proc/tty".into());
        file_opath("/proc", "filesystems", O_PATH, ResolverFlags::empty()) == Ok("/proc/filesystems".into());
        // ODIR          [cont](ENOTDIR) *2    ret         ENOTDIR
        sym_odir("/proc", "self", O_DIRECTORY, ResolverFlags::empty()) == Ok(format!("/proc/{}", syscalls::getpid()).into());
        dir_odir("/proc", "tty", O_DIRECTORY, ResolverFlags::empty()) == Ok("/proc/tty".into());
        file_odir("/proc", "filesystems", O_DIRECTORY, ResolverFlags::empty()) == Err(ErrorKind::OsError(Some(libc::ENOTDIR)));
        // OPATH|ODIR    [cont](ENOTDIR) *3    ret         ENOTDIR
        sym_opath_odir("/proc", "self", O_PATH|O_DIRECTORY, ResolverFlags::empty()) == Ok(format!("/proc/{}", syscalls::getpid()).into());
        dir_opath_odir("/proc", "tty", O_PATH|O_DIRECTORY, ResolverFlags::empty()) == Ok("/proc/tty".into());
        file_opath_odir("/proc", "filesystems", O_PATH|O_DIRECTORY, ResolverFlags::empty()) == Err(ErrorKind::OsError(Some(libc::ENOTDIR)));
        // ONF               ELOOP             ret           ret
        sym_onofollow("/proc", "self", O_NOFOLLOW, ResolverFlags::empty()) == Err(ErrorKind::OsError(Some(libc::ELOOP)));
        dir_onofollow("/proc", "tty", O_NOFOLLOW, ResolverFlags::empty()) == Ok("/proc/tty".into());
        file_onofollow("/proc", "filesystems", O_NOFOLLOW, ResolverFlags::empty()) == Ok("/proc/filesystems".into());
        // ONF|OPATH        ret-sym            ret           ret
        sym_opath_onofollow("/proc", "self", O_PATH|O_NOFOLLOW, ResolverFlags::empty()) == Ok("/proc/self".into());
        dir_opath_onofollow("/proc", "tty", O_PATH|O_NOFOLLOW, ResolverFlags::empty()) == Ok("/proc/tty".into());
        file_opath_onofollow("/proc", "filesystems", O_PATH|O_NOFOLLOW, ResolverFlags::empty()) == Ok("/proc/filesystems".into());
        // ONF|ODIR         ENOTDIR            ret         ENOTDIR
        sym_odir_onofollow("/proc", "self", O_DIRECTORY|O_NOFOLLOW, ResolverFlags::empty()) == Err(ErrorKind::OsError(Some(libc::ENOTDIR)));
        dir_odir_onofollow("/proc", "tty", O_DIRECTORY|O_NOFOLLOW, ResolverFlags::empty()) == Ok("/proc/tty".into());
        file_odir_onofollow("/proc", "filesystems", O_DIRECTORY|O_NOFOLLOW, ResolverFlags::empty()) == Err(ErrorKind::OsError(Some(libc::ENOTDIR)));
        // ONF|OPATH|ODIR   ENOTDIR            ret         EDOTDIR
        sym_opath_odir_onofollow("/proc", "self", O_PATH|O_DIRECTORY|O_NOFOLLOW, ResolverFlags::empty()) == Err(ErrorKind::OsError(Some(libc::ENOTDIR)));
        dir_opath_odir_onofollow("/proc", "tty", O_PATH|O_DIRECTORY|O_NOFOLLOW, ResolverFlags::empty()) == Ok("/proc/tty".into());
        file_opath_odir_onofollow("/proc", "filesystems", O_PATH|O_DIRECTORY|O_NOFOLLOW, ResolverFlags::empty()) == Err(ErrorKind::OsError(Some(libc::ENOTDIR)));
    }

    #[test]
    fn check_possible_magic_link() {
        // Regular symlinks.
        assert_matches!(super::check_possible_magic_link(Path::new("foo")), Ok(_));
        assert_matches!(super::check_possible_magic_link(Path::new("12345")), Ok(_));
        assert_matches!(
            super::check_possible_magic_link(Path::new("12345/foo/bar/baz")),
            Ok(_)
        );
        assert_matches!(
            super::check_possible_magic_link(Path::new("../../../../net/foo/bar")),
            Ok(_)
        );

        // Absolute symlinks.
        assert_matches!(super::check_possible_magic_link(Path::new("/")), Err(_));
        assert_matches!(
            super::check_possible_magic_link(Path::new("/foo/bar")),
            Err(_)
        );

        // anon-inode-like symlinks.
        assert_matches!(
            super::check_possible_magic_link(Path::new("user:[123456123123]")),
            Err(_)
        );
        assert_matches!(
            super::check_possible_magic_link(Path::new("pipe:[12345]")),
            Err(_)
        );
        assert_matches!(
            super::check_possible_magic_link(Path::new("anon_inode:[pidfd]")),
            Err(_)
        );
    }
}
