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

import typing
from typing import Any, IO, cast

# TODO: Remove this once we only support Python >= 3.11.
from typing_extensions import Self, TypeAlias

from ._internal import (
    # File type helpers.
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
    "PROC_ROOT",
    "PROC_SELF",
    "PROC_THREAD_SELF",
    "PROC_PID",
    "ProcfsHandle",
    # Shorthand for ProcfsHandle.cached().<foo>.
    "open",
    "open_raw",
    "readlink",
]

# TODO: Switch to "type ..." syntax once we switch to Python >= 3.12...?
ProcfsBase: TypeAlias = int

#: Resolve proc_* operations relative to the /proc root. Note that this mode
#: may be more expensive because we have to take steps to try to avoid leaking
#: unmasked procfs handles, so you should use PROC_SELF if you can.
PROC_ROOT: ProcfsBase = libpathrs_so.PATHRS_PROC_ROOT

#: Resolve proc_* operations relative to /proc/self. For most programs, this is
#: the standard choice.
PROC_SELF: ProcfsBase = libpathrs_so.PATHRS_PROC_SELF

#: Resolve proc_* operations relative to /proc/thread-self. In multi-threaded
#: programs where one thread has a different CLONE_FS, it is possible for
#: /proc/self to point the wrong thread and so /proc/thread-self may be
#: necessary.
PROC_THREAD_SELF: ProcfsBase = libpathrs_so.PATHRS_PROC_THREAD_SELF


def PROC_PID(pid: int) -> ProcfsBase:
    """
    Resolve proc_* operations relative to /proc/<pid>. Be aware that due to PID
    recycling, using this is generally not safe except in certain
    circumstances. Namely:

     * PID 1 (the init process), as that PID cannot ever get recycled.
     * Your current PID (though you should just use PROC_SELF).
     * PIDs of child processes (as long as you are sure that no other part of
       your program incorrectly catches or ignores SIGCHLD, and that you do it
       *before* you call wait(2)or any equivalent method that could reap
       zombies).
    """
    if pid & libpathrs_so.__PATHRS_PROC_TYPE_MASK:
        raise ValueError(f"invalid PROC_PID value {pid}")
    return libpathrs_so.__PATHRS_PROC_TYPE_PID | pid


class ProcfsHandle(WrappedFd):
    """ """

    _PROCFS_OPEN_HOW_TYPE = "pathrs_procfs_open_how *"

    @classmethod
    def cached(cls) -> Self:
        """
        Returns a cached version of ProcfsHandle that will always remain valid
        and cannot be closed. This is the recommended usage of ProcfsHandle.
        """
        return cls.from_raw_fd(libpathrs_so.PATHRS_PROC_DEFAULT_ROOTFD)

    @classmethod
    def new(cls, /, *, unmasked: bool = False) -> Self:
        """
        Create a new procfs handle with the requested configuration settings.

        Note that the requested configuration might be eligible for caching, in
        which case the ProcfsHandle.fileno() will contain a special sentinel
        value that cannot be used like a regular file descriptor.
        """

        # TODO: Is there a way to have ProcfsOpenHow actually subclass CData so
        # that we don't need to do any of these ugly casts?
        how = cast("libpathrs_so.ProcfsOpenHow", ffi.new(cls._PROCFS_OPEN_HOW_TYPE))
        how_size = ffi.sizeof(cast("Any", how))

        if unmasked:
            how.flags = libpathrs_so.PATHRS_PROCFS_NEW_UNMASKED

        fd = libpathrs_so.pathrs_procfs_open(how, how_size)
        if _is_pathrs_err(fd):
            raise PathrsError._fetch(fd) or INTERNAL_ERROR
        return cls.from_raw_fd(fd)

    def open_raw(self, base: ProcfsBase, path: str, flags: int, /) -> WrappedFd:
        """
        Open a procfs file using Unix open flags.

        This function returns a WrappedFd file handle.

        base indicates what the path should be relative to. Valid values
        include PROC_{ROOT,SELF,THREAD_SELF}.

        path is a relative path to base indicating which procfs file you wish
        to open.

        flags is the set of O_* flags you wish to pass to the open operation.
        If you do not intend to open a symlink, you should pass O_NOFOLLOW to
        flags to let libpathrs know that it can be more strict when opening the
        path.
        """
        # TODO: Should we default to O_NOFOLLOW or put a separate argument for it?
        fd = libpathrs_so.pathrs_proc_openat(self.fileno(), base, _cstr(path), flags)
        if _is_pathrs_err(fd):
            raise PathrsError._fetch(fd) or INTERNAL_ERROR
        return WrappedFd(fd)

    def open(
        self, base: ProcfsBase, path: str, mode: str = "r", /, *, extra_flags: int = 0
    ) -> IO[Any]:
        """
        Open a procfs file using Pythonic mode strings.

        This function returns an os.fdopen() file handle.

        base indicates what the path should be relative to. Valid values
        include PROC_{ROOT,SELF,THREAD_SELF}.

        path is a relative path to base indicating which procfs file you wish
        to open.

        mode is a Python mode string, and extra_flags can be used to indicate
        extra O_* flags you wish to pass to the open operation. If you do not
        intend to open a symlink, you should pass O_NOFOLLOW to extra_flags to
        let libpathrs know that it can be more strict when opening the path.
        """
        flags = _convert_mode(mode) | extra_flags
        with self.open_raw(base, path, flags) as file:
            return file.fdopen(mode)

    def readlink(self, base: ProcfsBase, path: str, /) -> str:
        """
        Fetch the target of a procfs symlink.

        Note that some procfs symlinks are "magic-links" where the returned
        string from readlink() is not how they are actually resolved.

        base indicates what the path should be relative to. Valid values
        include PROC_{ROOT,SELF,THREAD_SELF}.

        path is a relative path to base indicating which procfs file you wish
        to open.
        """
        # TODO: See if we can merge this with Root.readlink.
        cpath = _cstr(path)
        linkbuf_size = 128
        while True:
            linkbuf = _cbuffer(linkbuf_size)
            n = libpathrs_so.pathrs_proc_readlinkat(
                self.fileno(), base, cpath, linkbuf, linkbuf_size
            )
            if _is_pathrs_err(n):
                raise PathrsError._fetch(n) or INTERNAL_ERROR
            elif n <= linkbuf_size:
                buf = typing.cast(bytes, ffi.buffer(linkbuf, linkbuf_size)[:n])
                return buf.decode("latin1")
            else:
                # The contents were truncated. Unlike readlinkat, pathrs
                # returns the size of the link when it checked. So use the
                # returned size as a basis for the reallocated size (but in
                # order to avoid a DoS where a magic-link is growing by a
                # single byte each iteration, make sure we are a fair bit
                # larger).
                linkbuf_size += n


#: Open a procfs file (with unix open flags).
#: Shorthand for ProcfsHandle.cached().open(...).
open = ProcfsHandle.cached().open

#: Open a procfs file (with Pythonic mode strings).
#: Shorthand for ProcfsHandle.cached().open_raw(...).
open_raw = ProcfsHandle.cached().open_raw

#: Fetch the target of a procfs symlink.
#: Shorthand for ProcfsHandle.cached().readlink(...).
readlink = ProcfsHandle.cached().readlink
