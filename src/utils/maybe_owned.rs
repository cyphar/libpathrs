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

use std::os::unix::io::{AsFd, BorrowedFd, OwnedFd};

/// Like [`std::borrow::Cow`] but without the [`ToOwned`] requirement, and only
/// for file descriptors.
///
/// This is mainly useful when you need to write a function that takes something
/// equivalent to `Option<BorrowedFd<'_>>` and opens an alternative [`OwnedFd`]
/// (or any other `Fd: AsFd`) if passed [`None`]. Normally you cannot really do
/// this smoothly.
///
/// Note that due to Rust's temporaries handling and restrictions of the
/// [`AsFd`] trait, you need to do something like the following:
///
/// ```ignore
/// fn procfs_foobar(fd: Option<BorrowedFd<'_>>) -> Result<(), Error> {
///     let fd = match fd {
///         None => MaybeOwnedFd::OwnedFd(File::open("/proc")?),
///         Some(fd) => MaybeOwnedFd::BorrowedFd(fd),
///     };
///     let fd = fd.as_fd(); // BorrowedFd<'_>
///     // do something with fd
/// }
/// ```
///
/// This will give you a [`BorrowedFd`] with minimal fuss.
///
/// [`OwnedFd`]: std::os::unix::io::OwnedFd
/// [`ToOwned`]: std::borrow::ToOwned
#[derive(Debug)]
pub(crate) enum MaybeOwnedFd<'fd, Fd>
where
    Fd: AsFd,
{
    OwnedFd(Fd),
    BorrowedFd(BorrowedFd<'fd>),
}

impl<'fd> From<OwnedFd> for MaybeOwnedFd<'fd, OwnedFd> {
    fn from(fd: OwnedFd) -> Self {
        Self::OwnedFd(fd)
    }
}

impl<'fd, Fd> From<BorrowedFd<'fd>> for MaybeOwnedFd<'fd, Fd>
where
    Fd: AsFd,
{
    fn from(fd: BorrowedFd<'fd>) -> Self {
        Self::BorrowedFd(fd)
    }
}

// I wish we could make this "impl AsFd for MaybeOwnedFd" but the lifetimes
// don't match, even though it really feels like it should be possible.
impl<'fd, Fd> MaybeOwnedFd<'fd, Fd>
where
    Fd: AsFd,
{
    /// Unwrap `MaybeOwnedFd` into the `OwnedFd` variant if possible.
    pub(crate) fn into_owned(self) -> Option<Fd> {
        match self {
            Self::OwnedFd(fd) => Some(fd),
            Self::BorrowedFd(_) => None,
        }
    }

    /// Very similar in concept to [`AsFd::as_fd`] but with some additional
    /// lifetime restrictions that make it incompatible with [`AsFd`].
    pub(crate) fn as_fd<'a>(&'a self) -> BorrowedFd<'a>
    where
        'a: 'fd,
    {
        match self {
            Self::OwnedFd(fd) => fd.as_fd(),
            Self::BorrowedFd(fd) => fd.as_fd(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{
        fs::File,
        os::unix::io::{AsFd, AsRawFd, OwnedFd},
    };

    use anyhow::Error;
    use pretty_assertions::{assert_eq, assert_matches};

    #[test]
    fn as_fd() -> Result<(), Error> {
        let f: OwnedFd = File::open(".")?.into();
        let fd = f.as_raw_fd();
        let owned: MaybeOwnedFd<OwnedFd> = f.into();
        assert_matches!(owned, MaybeOwnedFd::OwnedFd(_));
        assert_eq!(owned.as_fd().as_raw_fd(), fd);
        assert_matches!(owned.into_owned(), Some(_));

        let f = File::open(".")?;
        let borrowed: MaybeOwnedFd<OwnedFd> = f.as_fd().into();
        assert_matches!(borrowed, MaybeOwnedFd::BorrowedFd(_));
        assert_matches!(borrowed.into_owned(), None);

        Ok(())
    }

    #[test]
    fn into_owned() -> Result<(), Error> {
        let f: OwnedFd = File::open(".")?.into();
        let owned: MaybeOwnedFd<OwnedFd> = f.into();
        assert_matches!(
            owned,
            MaybeOwnedFd::OwnedFd(_),
            "MaybeOwnedFd::from(OwnedFd)"
        );
        assert_matches!(
            owned.into_owned(),
            Some(_),
            "MaybeOwnedFd::from(OwnedFd).into_owned()"
        );

        let f = File::open(".")?;
        let borrowed: MaybeOwnedFd<OwnedFd> = f.as_fd().into();
        assert_matches!(
            borrowed,
            MaybeOwnedFd::BorrowedFd(_),
            "MaybeOwnedFd::from(BorrowedFd)"
        );
        assert_matches!(
            borrowed.into_owned(),
            None,
            "MaybeOwnedFd::from(BorrowedFd).into_owned()"
        );

        Ok(())
    }
}
