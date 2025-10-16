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

import io
import os
import sys
import copy
import errno
import fcntl

import typing
from types import TracebackType
from typing import Any, Dict, IO, Optional, TextIO, Type, TypeVar, Union

# TODO: Remove this once we only support Python >= 3.11.
from typing_extensions import Self, TypeAlias

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


def _cstr(pystr: str) -> CString:
    return ffi.new("char[]", pystr.encode("utf8"))


def _pystr(cstr: CString) -> str:
    s = ffi.string(cstr)
    assert isinstance(s, bytes)  # typing
    return s.decode("utf8")


def _cbuffer(size: int) -> CBuffer:
    return ffi.new("char[%d]" % (size,))


def _is_pathrs_err(ret: int) -> bool:
    return ret < libpathrs_so.__PATHRS_MAX_ERR_VALUE


class PathrsError(Exception):
    """
    Represents a libpathrs error. All libpathrs errors have a description
    (PathrsError.message) and errors that were caused by an underlying OS error
    (or can be translated to an OS error) also include the errno value
    (PathrsError.errno).
    """

    message: str
    errno: Optional[int]
    strerror: Optional[str]

    def __init__(self, message: str, /, *, errno: Optional[int] = None):
        # Construct Exception.
        super().__init__(message)

        # Basic arguments.
        self.message = message
        self.errno = errno

        # Pre-format the errno.
        self.strerror = None
        if errno is not None:
            try:
                self.strerror = os.strerror(errno)
            except ValueError:
                self.strerror = str(errno)

    @classmethod
    def _fetch(cls, err_id: int, /) -> Optional[Self]:
        if err_id >= 0:
            return None

        err = libpathrs_so.pathrs_errorinfo(err_id)
        if err == ffi.NULL:  # type: ignore # TODO: Make this check nicer...
            return None

        description = _pystr(err.description)
        errno = err.saved_errno or None

        # TODO: Should we use ffi.gc()? mypy doesn't seem to like our types...
        libpathrs_so.pathrs_errorinfo_free(err)
        del err

        return cls(description, errno=errno)

    def __str__(self) -> str:
        if self.errno is None:
            return self.message
        else:
            return "%s (%s)" % (self.message, self.strerror)

    def __repr__(self) -> str:
        return "Error(%r, errno=%r)" % (self.message, self.errno)

    def pprint(self, out: TextIO = sys.stdout) -> None:
        "Pretty-print the error to the given @out file."
        # Basic error information.
        if self.errno is None:
            print("pathrs error:", file=out)
        else:
            print("pathrs error [%s]:" % (self.strerror,), file=out)
        print("  %s" % (self.message,), file=out)


INTERNAL_ERROR = PathrsError("tried to fetch libpathrs error but no error found")


class FilenoFile(typing.Protocol):
    def fileno(self) -> int: ...


FileLike = Union[FilenoFile, int]


def _fileno(file: FileLike) -> int:
    if isinstance(file, int):
        # file is a plain fd
        return file
    else:
        # Assume there is a fileno method.
        return file.fileno()


def _clonefile(file: FileLike) -> int:
    return fcntl.fcntl(_fileno(file), fcntl.F_DUPFD_CLOEXEC)


# TODO: Switch to def foo[T](...): ... syntax with Python >= 3.12.
Fd = TypeVar("Fd", bound="WrappedFd")


class WrappedFd(object):
    """
    Represents a file descriptor that allows for manual lifetime management,
    unlike os.fdopen() which are tracked by the GC with no way of "leaking" the
    file descriptor for FFI purposes.

    pathrs will return WrappedFds for most operations that return an fd.
    """

    _fd: Optional[int]

    def __init__(self, file: FileLike, /):
        """
        Construct a WrappedFd from any file-like object.

        For most cases, the WrappedFd will take ownership of the lifetime of
        the file handle. This means you should  So a raw file descriptor must
        only be turned into a WrappedFd *once* (unless you make sure to use
        WrappedFd.leak() to ensure there is only ever one owner of the handle
        at a given time).

        However, for os.fdopen() (or similar Pythonic file objects that are
        tracked by the GC), we have to create a clone and so the WrappedFd is a
        copy.
        """
        # TODO: Maybe we should always clone to make these semantics less
        # confusing...?
        fd = _fileno(file)
        if isinstance(file, io.IOBase):
            # If this is a regular open file, we need to make a copy because
            # you cannot leak files and so the GC might close it from
            # underneath us.
            fd = _clonefile(fd)
        self._fd = fd

    def fileno(self) -> int:
        """
        Return the file descriptor number of this WrappedFd.

        Note that the file can still be garbage collected by Python after this
        call, so the file descriptor number might become invalid (or worse, be
        reused for an unrelated file).

        If you want to convert a WrappedFd to a file descriptor number and stop
        the GC from the closing the file, use WrappedFd.into_raw_fd().
        """
        if self._fd is None:
            raise OSError(errno.EBADF, "Closed file descriptor")
        return self._fd

    def leak(self) -> None:
        """
        Clears this WrappedFd without closing the underlying file, to stop GC
        from closing the file.

        Note that after this operation, all operations on this WrappedFd will
        return an error. If you want to get the underlying file handle and then
        leak the WrappedFd, just use WrappedFd.into_raw_fd() which does both
        for you.
        """
        if self._fd is not None and self._fd >= 0:
            self._fd = None

    def fdopen(self, mode: str = "r") -> IO[Any]:
        """
        Convert this WrappedFd into an os.fileopen() handle.

        This operation implicitly calls WrappedFd.leak(), so the WrappedFd will
        no longer be useful and you should instead use the returned os.fdopen()
        handle.
        """
        fd = self.fileno()
        try:
            file = os.fdopen(fd, mode)
            self.leak()
            return file
        except:
            # "Unleak" the file if there was an error.
            self._fd = fd
            raise

    @classmethod
    def from_raw_fd(cls: Type[Fd], fd: int, /) -> Fd:
        "Shorthand for WrappedFd(fd)."
        return cls(fd)

    @classmethod
    def from_file(cls: Type[Fd], file: FileLike, /) -> Fd:
        "Shorthand for WrappedFd(file)."
        return cls(file)

    def into_raw_fd(self) -> int:
        """
        Convert this WrappedFd into a raw file descriptor that GC won't touch.

        This is just shorthand for WrappedFd.fileno() to get the fileno,
        followed by WrappedFd.leak().
        """
        fd = self.fileno()
        self.leak()
        return fd

    def isclosed(self) -> bool:
        """
        Returns whether the underlying file descriptor is closed or the
        WrappedFd has been leaked.
        """
        return self._fd is None

    def close(self) -> None:
        """
        Manually close the underlying file descriptor for this WrappedFd.

        WrappedFds are garbage collected, so this is usually unnecessary unless
        you really care about the point where a file is closed.
        """
        if not self.isclosed():
            assert self._fd is not None  # typing
            if self._fd >= 0:
                os.close(self._fd)
                self._fd = None

    def clone(self) -> Self:
        "Create a clone of this WrappedFd that has a separate lifetime."
        if self.isclosed():
            raise ValueError("cannot clone closed file")
        assert self._fd is not None  # typing
        newfd = self._fd
        if self._fd >= 0:
            newfd = _clonefile(self._fd)
        return self.__class__.from_raw_fd(newfd)

    def __copy__(self) -> Self:
        "Identical to WrappedFd.clone()"
        # A "shallow copy" of a file is the same as a deep copy.
        return copy.deepcopy(self)

    def __deepcopy__(self, memo: Dict[int, Any]) -> Self:
        "Identical to WrappedFd.clone()"
        return self.clone()

    def __del__(self) -> None:
        "Identical to WrappedFd.close()"
        self.close()

    def __enter__(self) -> Self:
        return self

    def __exit__(
        self,
        exc_type: Optional[Type[BaseException]],
        exc_value: Optional[BaseException],
        exc_traceback: Optional[TracebackType],
    ) -> None:
        self.close()


# XXX: This is _super_ ugly but so is the one in CPython.
def _convert_mode(mode: str) -> int:
    mode_set = set(mode)
    flags = os.O_CLOEXEC

    # We don't support O_CREAT or O_EXCL with libpathrs -- use creat().
    if "x" in mode_set:
        raise ValueError("pathrs doesn't support mode='x', use Root.creat()")
    # Basic sanity-check to make sure we don't accept garbage modes.
    if len(mode_set & {"r", "w", "a"}) > 1:
        raise ValueError("must have exactly one of read/write/append mode")

    read = False
    write = False

    if "+" in mode_set:
        read = True
        write = True
    if "r" in mode_set:
        read = True
    if "w" in mode_set:
        write = True
        flags |= os.O_TRUNC
    if "a" in mode_set:
        write = True
        flags |= os.O_APPEND

    if read and write:
        flags |= os.O_RDWR
    elif write:
        flags |= os.O_WRONLY
    else:
        flags |= os.O_RDONLY

    # We don't care about "b" or "t" since that's just a Python thing.
    return flags
