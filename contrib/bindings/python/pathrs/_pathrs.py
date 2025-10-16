#!/usr/bin/python3
# SPDX-License-Identifier: MPL-2.0
#
# libpathrs: safe path resolution on Linux
# Copyright (C) 2019-2025 Aleksa Sarai <cyphar@cyphar.com>
# Copyright (C) 2019-2025 SUSE LLC
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

import os

import typing
from typing import Any, IO, Union

# TODO: Remove this once we only support Python >= 3.11.
from typing_extensions import TypeAlias

from ._internal import (
    # File type helpers.
    FileLike,
    WrappedFd,
    _convert_mode,
    # Error API.
    PathrsError,
    _is_pathrs_err,
    INTERNAL_ERROR,
    # CFFI helpers.
    _cstr,
    _cbuffer,
)
from ._libpathrs_cffi import lib as libpathrs_so

if typing.TYPE_CHECKING:
    # mypy apparently cannot handle the "ffi: cffi.api.FFI" definition in
    # _libpathrs_cffi/__init__.pyi so we need to explicitly reference the type
    # from cffi here.
    import cffi

    ffi = cffi.FFI()
    CString: TypeAlias = cffi.FFI.CData
    CBuffer: TypeAlias = cffi.FFI.CData
else:
    from ._libpathrs_cffi import ffi

    CString: TypeAlias = ffi.CData
    CBuffer: TypeAlias = ffi.CData

__all__ = [
    # Core api.
    "Root",
    "Handle",
    # Error api (re-export).
    "PathrsError",
]


class Handle(WrappedFd):
    "A handle to a filesystem object, usually resolved using Root.resolve()."

    def reopen(self, mode: str = "r", /, *, extra_flags: int = 0) -> IO[Any]:
        """
        Upgrade a Handle to a os.fdopen() file handle.

        mode is a Python mode string, and extra_flags can be used to indicate
        extra O_* flags you wish to pass to the reopen operation.

        The returned file handle is independent to the original Handle, and you
        can freely call Handle.reopen() on the same Handle multiple times.
        """
        flags = _convert_mode(mode) | extra_flags
        with self.reopen_raw(flags) as file:
            return file.fdopen(mode)

    def reopen_raw(self, flags: int, /) -> WrappedFd:
        """
        Upgrade a Handle to a WrappedFd file handle.

        flags is the set of O_* flags you wish to pass to the open operation.

        The returned file handle is independent to the original Handle, and you
        can freely call Handle.reopen() on the same Handle multiple times.
        """
        fd = libpathrs_so.pathrs_reopen(self.fileno(), flags)
        if _is_pathrs_err(fd):
            raise PathrsError._fetch(fd) or INTERNAL_ERROR
        return WrappedFd(fd)


class Root(WrappedFd):
    """
    A handle to a filesystem root, which filesystem operations are all done
    relative to.
    """

    def __init__(self, file_or_path: Union[FileLike, str], /):
        """
        Create a handle from a file-like object or a path to a directory.

        Note that creating a Root in an attacker-controlled directory can allow
        for an attacker to trick you into allowing breakouts. If file_or_path
        is a path string, be aware there are no protections against rename race
        attacks when opening the Root directory handle itself.
        """
        if isinstance(file_or_path, str):
            path = _cstr(file_or_path)
            fd = libpathrs_so.pathrs_open_root(path)
            if _is_pathrs_err(fd):
                raise PathrsError._fetch(fd) or INTERNAL_ERROR
            file: FileLike = fd
        else:
            file = file_or_path

        # XXX: Is this necessary?
        super().__init__(file)

    def resolve(self, path: str, /, *, follow_trailing: bool = True) -> Handle:
        """
        Resolve the given path inside the Root and return a Handle.

        follow_trailing indicates what resolve should do if the final component
        of the path is a symlink. The default is to continue resolving it, if
        follow_trailing=False then a handle to the symlink itself is returned.
        This has some limited uses, but most users should use the default.

        A pathrs.Error is raised if the path doesn't exist.
        """
        if follow_trailing:
            fd = libpathrs_so.pathrs_inroot_resolve(self.fileno(), _cstr(path))
        else:
            fd = libpathrs_so.pathrs_inroot_resolve_nofollow(self.fileno(), _cstr(path))
        if _is_pathrs_err(fd):
            raise PathrsError._fetch(fd) or INTERNAL_ERROR
        return Handle(fd)

    def open(
        self,
        path: str,
        mode: str = "r",
        /,
        *,
        follow_trailing: bool = True,
        extra_flags: int = 0,
    ) -> IO[Any]:
        """
        Resolve and open a path inside the Root and return an os.fdopen() file
        handle.

        This is effectively shorthand for Root.resolve(path).reopen(...), but
        for the openat2-based resolver this is slightly more efficient if you
        just want to open a file and don't need to do multiple reopens of the
        Handle.

        mode is a Python mode string, and extra_flags can be used to indicate
        extra O_* flags you wish to pass to the open operation.

        follow_trailing has the same behaviour as Root.resolve(), and is
        equivalent to passing os.O_NOFOLLOW to extra_flags.
        """
        flags = _convert_mode(mode) | extra_flags
        if not follow_trailing:
            flags |= os.O_NOFOLLOW
        with self.open_raw(path, flags) as file:
            return file.fdopen(mode)

    def open_raw(self, path: str, flags: int, /) -> WrappedFd:
        """
        Resolve and open a path inside the Root and return a WrappedFd file
        handle.

        This is effectively shorthand for Root.resolve(path).reopen_raw(flags),
        but for the openat2-based resolver this is slightly more efficient if
        you just want to open a file and don't need to do multiple reopens of
        the Handle.

        If flags contains os.O_NOFOLLOW, then the resolution is done as if you
        passed follow_trailing=False to Root.resolve().
        """
        fd = libpathrs_so.pathrs_inroot_open(self.fileno(), _cstr(path), flags)
        if _is_pathrs_err(fd):
            raise PathrsError._fetch(fd) or INTERNAL_ERROR
        return WrappedFd(fd)

    def readlink(self, path: str, /) -> str:
        """
        Fetch the target of a symlink at the given path in the Root.

        A pathrs.Error is raised if the path is not a symlink or doesn't exist.
        """
        cpath = _cstr(path)
        linkbuf_size = 128
        while True:
            linkbuf = _cbuffer(linkbuf_size)
            n = libpathrs_so.pathrs_inroot_readlink(
                self.fileno(), cpath, linkbuf, linkbuf_size
            )
            if _is_pathrs_err(n):
                raise PathrsError._fetch(n) or INTERNAL_ERROR
            elif n <= linkbuf_size:
                buf = typing.cast(bytes, ffi.buffer(linkbuf, linkbuf_size)[:n])
                return buf.decode("latin1")
            else:
                # The contents were truncated. Unlike readlinkat, pathrs returns
                # the size of the link when it checked. So use the returned size
                # as a basis for the reallocated size (but in order to avoid a DoS
                # where a magic-link is growing by a single byte each iteration,
                # make sure we are a fair bit larger).
                linkbuf_size += n

    def creat(
        self, path: str, mode: str = "r", filemode: int = 0o644, /, extra_flags: int = 0
    ) -> IO[Any]:
        """
        Atomically create-and-open a new file at the given path in the Root,
        a-la O_CREAT.

        This method returns an os.fdopen() file handle.

        filemode is the Unix DAC mode you wish the new file to be created with.
        This mode might not be the actual mode of the created file due to a
        variety of external factors (umask, setgid bits, POSIX ACLs).

        mode is a Python mode string, and extra_flags can be used to indicate
        extra O_* flags you wish to pass to the reopen operation. If you wish
        to ensure the new file was created *by you* then you may wish to add
        O_EXCL to extra_flags.
        """
        flags = _convert_mode(mode) | extra_flags
        fd = libpathrs_so.pathrs_inroot_creat(
            self.fileno(), _cstr(path), flags, filemode
        )
        if _is_pathrs_err(fd):
            raise PathrsError._fetch(fd) or INTERNAL_ERROR
        return os.fdopen(fd, mode)

    def creat_raw(self, path: str, flags: int, filemode: int = 0o644, /) -> WrappedFd:
        """
        Atomically create-and-open a new file at the given path in the Root,
        a-la O_CREAT.

        This method returns a WrappedFd handle.

        filemode is the Unix DAC mode you wish the new file to be created with.
        This mode might not be the actual mode of the created file due to a
        variety of external factors (umask, setgid bits, POSIX ACLs).

        flags is the set of O_* flags you wish to pass to the open operation. If
        you do not intend to open a symlink, you should pass O_NOFOLLOW to flags to
        let libpathrs know that it can be more strict when opening the path.
        """
        fd = libpathrs_so.pathrs_inroot_creat(
            self.fileno(), _cstr(path), flags, filemode
        )
        if _is_pathrs_err(fd):
            raise PathrsError._fetch(fd) or INTERNAL_ERROR
        return WrappedFd(fd)

    def rename(self, src: str, dst: str, flags: int = 0, /) -> None:
        """
        Rename a path from src to dst within the Root.

        flags can be any renameat2(2) flags you wish to use, which can change
        the behaviour of this method substantially. For instance,
        RENAME_EXCHANGE will turn this into an atomic swap operation.
        """
        # TODO: Should we have a separate Root.swap() operation?
        err = libpathrs_so.pathrs_inroot_rename(
            self.fileno(), _cstr(src), _cstr(dst), flags
        )
        if _is_pathrs_err(err):
            raise PathrsError._fetch(err) or INTERNAL_ERROR

    def rmdir(self, path: str, /) -> None:
        """
        Remove an empty directory at the given path within the Root.

        To remove non-empty directories recursively, you can use
        Root.remove_all().
        """
        err = libpathrs_so.pathrs_inroot_rmdir(self.fileno(), _cstr(path))
        if _is_pathrs_err(err):
            raise PathrsError._fetch(err) or INTERNAL_ERROR

    def unlink(self, path: str, /) -> None:
        """
        Remove a non-directory inode at the given path within the Root.

        To remove empty directories, you can use Root.remove_all(). To remove
        files and non-empty directories recursively, you can use
        Root.remove_all().
        """
        err = libpathrs_so.pathrs_inroot_unlink(self.fileno(), _cstr(path))
        if _is_pathrs_err(err):
            raise PathrsError._fetch(err) or INTERNAL_ERROR

    def remove_all(self, path: str, /) -> None:
        """
        Remove the file or directory (empty or non-empty) at the given path
        within the Root.
        """
        err = libpathrs_so.pathrs_inroot_remove_all(self.fileno(), _cstr(path))
        if _is_pathrs_err(err):
            raise PathrsError._fetch(err) or INTERNAL_ERROR

    def mkdir(self, path: str, mode: int, /) -> None:
        """
        Create a directory at the given path within the Root.

        mode is the Unix DAC mode you wish the new directory to be created
        with. This mode might not be the actual mode of the created file due to
        a variety of external factors (umask, setgid bits, POSIX ACLs).

        A pathrs.Error will be raised if the parent directory doesn't exist, or
        the path already exists. To create a directory and all of its parent
        directories (or just reuse an existing directory) you can use
        Root.mkdir_all().
        """
        err = libpathrs_so.pathrs_inroot_mkdir(self.fileno(), _cstr(path), mode)
        if _is_pathrs_err(err):
            raise PathrsError._fetch(err) or INTERNAL_ERROR

    def mkdir_all(self, path: str, mode: int, /) -> Handle:
        """
        Recursively create a directory and all of its parents at the given path
        within the Root (or reuse an existing directory if the path already
        exists).

        This method returns a Handle to the created directory.

        mode is the Unix DAC mode you wish any new directories to be created
        with. This mode might not be the actual mode of the created file due to
        a variety of external factors (umask, setgid bits, POSIX ACLs). If the
        full path already exists, this mode is ignored and the existing
        directory mode is kept.
        """
        fd = libpathrs_so.pathrs_inroot_mkdir_all(self.fileno(), _cstr(path), mode)
        if _is_pathrs_err(fd):
            raise PathrsError._fetch(fd) or INTERNAL_ERROR
        return Handle(fd)

    def mknod(self, path: str, mode: int, device: int = 0, /) -> None:
        """
        Create a new inode at the given path within the Root.

        mode both indicates the file type (it must contain a valid bit from
        S_IFMT to indicate what kind of file to create) and what the mode of
        the newly created file should have. This mode might not be the actual
        mode of the created file due to a variety of external factors (umask,
        setgid bits, POSIX ACLs).

        dev is the the (major, minor) device number used for the new inode if
        the mode contains S_IFCHR or S_IFBLK. You can construct the device
        number from a (major, minor) using os.makedev().

        A pathrs.Error is raised if the path already exists.
        """
        err = libpathrs_so.pathrs_inroot_mknod(self.fileno(), _cstr(path), mode, device)
        if _is_pathrs_err(err):
            raise PathrsError._fetch(err) or INTERNAL_ERROR

    def hardlink(self, path: str, target: str, /) -> None:
        """
        Create a hardlink between two paths inside the Root.

        path is the path to the *new* hardlink, and target is a path to the
        *existing* file.

        A pathrs.Error is raised if the path for the new hardlink already
        exists.
        """
        err = libpathrs_so.pathrs_inroot_hardlink(
            self.fileno(), _cstr(path), _cstr(target)
        )
        if _is_pathrs_err(err):
            raise PathrsError._fetch(err) or INTERNAL_ERROR

    def symlink(self, path: str, target: str, /) -> None:
        """
        Create a symlink at the given path in the Root.

        path is the path to the *new* symlink, and target is what the symink
        will point to. Note that symlinks contents are not verified on Linux,
        so there are no restrictions on what target you put.

        A pathrs.Error is raised if the path for the new symlink already
        exists.
        """
        err = libpathrs_so.pathrs_inroot_symlink(
            self.fileno(), _cstr(path), _cstr(target)
        )
        if _is_pathrs_err(err):
            raise PathrsError._fetch(err) or INTERNAL_ERROR
