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

use crate::{
    error::{Error, ErrorExt, ErrorImpl},
    flags::OpenFlags,
    syscalls,
};

use std::{
    ffi::OsStr,
    os::unix::{ffi::OsStrExt, io::AsFd},
    path::Path,
};

use rustix::fs::{AtFlags, Dir};

trait RmdirResultExt {
    // ENOENT from a removal function should be treated the same as an Ok(()).
    fn ignore_enoent(self) -> Self;
}

impl RmdirResultExt for Result<(), Error> {
    fn ignore_enoent(self) -> Self {
        match self.map_err(|err| (err.kind().errno(), err)) {
            Ok(()) | Err((Some(libc::ENOENT), _)) => Ok(()),
            Err((_, err)) => Err(err),
        }
    }
}

fn remove_inode(dirfd: impl AsFd, name: impl AsRef<Path>) -> Result<(), Error> {
    let dirfd = dirfd.as_fd();
    let name = name.as_ref();

    // To ensure we return a useful error, we try both unlink and rmdir and
    // try to avoid returning EISDIR/ENOTDIR if both failed.
    syscalls::unlinkat(dirfd, name, AtFlags::empty())
        .or_else(|unlink_err| {
            syscalls::unlinkat(dirfd, name, AtFlags::REMOVEDIR).map_err(|rmdir_err| {
                if rmdir_err.root_cause().raw_os_error() == Some(libc::ENOTDIR) {
                    unlink_err
                } else {
                    rmdir_err
                }
            })
        })
        .map_err(|err| {
            ErrorImpl::RawOsError {
                operation: "remove inode".into(),
                source: err,
            }
            .into()
        })
}

pub(crate) fn remove_all(dirfd: impl AsFd, name: impl AsRef<Path>) -> Result<(), Error> {
    let dirfd = dirfd.as_fd();
    let name = name.as_ref();

    if name.as_os_str().as_bytes().contains(&b'/') {
        Err(ErrorImpl::SafetyViolation {
            description: "remove_all reached a component containing '/'".into(),
        })?;
    }

    // Fast path -- try to remove it with unlink/rmdir.
    if remove_inode(dirfd, name).ignore_enoent().is_ok() {
        return Ok(());
    }

    // Try to delete all children. We need to re-do the iteration until there
    // are no components left because deleting entries while iterating over a
    // directory can lead to the iterator skipping components. An attacker could
    // try to make this loop forever by consistently creating inodes, but
    // there's not much we can do about it and I suspect they would eventually
    // lose the race.
    let subdir = match syscalls::openat(dirfd, name, OpenFlags::O_DIRECTORY, 0).map_err(|err| {
        ErrorImpl::RawOsError {
            operation: "open directory to scan entries".into(),
            source: err,
        }
    }) {
        Ok(fd) => fd,
        Err(err) => match err.kind().errno() {
            // The path was deleted between us trying to with remove_inode() and
            // now -- just return as if we were the ones that deleted it.
            Some(libc::ENOENT) => return Ok(()),
            _ => Err(err)?,
        },
    };
    loop {
        // TODO: Dir creates a new file descriptor rather than reusing the one
        //       we have, and RawDir can't be used as an Iterator yet (rustix
        //       needs GAT to make that work). But this is okay for now...
        let mut iter = match Dir::read_from(&subdir)
            .map_err(|err| ErrorImpl::OsError {
                operation: "create directory iterator".into(),
                source: err.into(),
            })
            .with_wrap(|| format!("scan directory {name:?} for deletion"))
        {
            Ok(iter) => iter,
            Err(err) => match err.kind().errno() {
                // If we got ENOENT that means the directory got deleted after
                // we opened it, so stop iterating (maybe another thread did "rm
                // -rf"). An attacker might've also replaced the directory but
                // we're not going retry opening it because that could lead to a
                // DoS. remove_inode will error out in that case, and that's
                // fine.
                Some(libc::ENOENT) => break,
                // TODO: Maybe we want to just break out of the loop here as
                //       well, rather than return an error? If remove_inode()
                //       again succeeds we're golden.
                _ => Err(err)?,
            },
        }
        .filter(|res| {
            !matches!(
                res.as_ref().map(|dentry| dentry.file_name().to_bytes()),
                Ok(b".") | Ok(b"..")
            )
        })
        .peekable();

        // We can stop iterating when a fresh directory iterator is empty.
        if iter.peek().is_none() {
            break;
        }

        // Recurse into all of the children and try to delete them.
        for child in iter {
            // TODO: We probably want to break out of the scan loop here if this
            //       is an error as well.
            let child = child.map_err(|err| ErrorImpl::OsError {
                operation: format!("scan directory {name:?}").into(),
                source: err.into(),
            })?;
            let name: &Path = OsStr::from_bytes(child.file_name().to_bytes()).as_ref();
            remove_all(&subdir, name).ignore_enoent()?
        }
    }

    // We have deleted all of the children of the directory, let's try to delete
    // the inode again (it should be empty now -- an attacker could add things
    // but we can just error out in that case, and if they swapped it to a file
    // then remove_inode will take care of that).
    remove_inode(dirfd, name)
        .ignore_enoent()
        .with_wrap(|| format!("deleting emptied directory {name:?}"))
}

#[cfg(test)]
mod tests {
    use super::remove_all;
    use crate::{error::ErrorKind, tests::common as tests_common, Root};

    use std::{os::unix::io::OwnedFd, path::Path};

    use anyhow::Error;
    use pretty_assertions::assert_eq;

    #[test]
    fn remove_all_basic() -> Result<(), Error> {
        let dir = tests_common::create_basic_tree()?;
        let dirfd: OwnedFd = Root::open(&dir)?.into();

        assert_eq!(
            remove_all(&dirfd, Path::new("a")).map_err(|err| err.kind()),
            Ok(()),
            "removeall(root, 'a') should work",
        );
        assert_eq!(
            remove_all(&dirfd, Path::new("b")).map_err(|err| err.kind()),
            Ok(()),
            "removeall(root, 'b') should work",
        );
        assert_eq!(
            remove_all(&dirfd, Path::new("c")).map_err(|err| err.kind()),
            Ok(()),
            "removeall(root, 'c') should work",
        );

        let _dir = dir; // make sure the tempdir is not dropped early
        Ok(())
    }

    #[test]
    fn remove_all_slash_path() -> Result<(), Error> {
        let dir = tests_common::create_basic_tree()?;
        let dirfd: OwnedFd = Root::open(&dir)?.into();

        assert_eq!(
            remove_all(&dirfd, Path::new("/")).map_err(|err| err.kind()),
            Err(ErrorKind::SafetyViolation),
            "removeall(root, '/') should fail",
        );
        assert_eq!(
            remove_all(&dirfd, Path::new("./a")).map_err(|err| err.kind()),
            Err(ErrorKind::SafetyViolation),
            "removeall(root, './a') should fail",
        );
        assert_eq!(
            remove_all(&dirfd, Path::new("a/")).map_err(|err| err.kind()),
            Err(ErrorKind::SafetyViolation),
            "removeall(root, 'a/') should fail",
        );

        let _dir = dir; // make sure the tempdir is not dropped early
        Ok(())
    }
}
