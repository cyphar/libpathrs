# SPDX-License-Identifier: MPL-2.0
#
# libpathrs: safe path resolution on Linux
# Copyright (C) 2019-2025 Aleksa Sarai <cyphar@cyphar.com>
# Copyright (C) 2019-2025 SUSE LLC
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

from typing import type_check_only, Union

# TODO: Remove this once we only support Python >= 3.10.
from typing_extensions import TypeAlias, Literal

from .._pathrs import CBuffer, CString
from ..procfs import ProcfsBase

RawFd: TypeAlias = int

# pathrs_errorinfo_t *
@type_check_only
class CError:
    saved_errno: int
    description: CString

ErrorId: TypeAlias = int
__PATHRS_MAX_ERR_VALUE: ErrorId

# TODO: We actually return Union[CError, cffi.FFI.NULL] but we can't express
#       this using the typing stubs for CFFI...
def pathrs_errorinfo(err_id: Union[ErrorId, int]) -> CError: ...
def pathrs_errorinfo_free(err: CError) -> None: ...

# uint64_t
ProcfsOpenFlags: TypeAlias = int
PATHRS_PROCFS_NEW_UNMASKED: ProcfsOpenFlags

# pathrs_procfs_open_how *
@type_check_only
class ProcfsOpenHow:
    flags: ProcfsOpenFlags

PATHRS_PROC_ROOT: ProcfsBase
PATHRS_PROC_SELF: ProcfsBase
PATHRS_PROC_THREAD_SELF: ProcfsBase

__PATHRS_PROC_TYPE_MASK: ProcfsBase
__PATHRS_PROC_TYPE_PID: ProcfsBase

PATHRS_PROC_DEFAULT_ROOTFD: RawFd

# procfs API
def pathrs_procfs_open(how: ProcfsOpenHow, size: int) -> Union[RawFd, ErrorId]: ...
def pathrs_proc_open(
    base: ProcfsBase, path: CString, flags: int
) -> Union[RawFd, ErrorId]: ...
def pathrs_proc_openat(
    proc_root_fd: RawFd, base: ProcfsBase, path: CString, flags: int
) -> Union[RawFd, ErrorId]: ...
def pathrs_proc_readlink(
    base: ProcfsBase, path: CString, linkbuf: CBuffer, linkbuf_size: int
) -> Union[int, ErrorId]: ...
def pathrs_proc_readlinkat(
    proc_root_fd: RawFd,
    base: ProcfsBase,
    path: CString,
    linkbuf: CBuffer,
    linkbuf_size: int,
) -> Union[int, ErrorId]: ...

# core API
def pathrs_open_root(path: CString) -> Union[RawFd, ErrorId]: ...
def pathrs_reopen(fd: RawFd, flags: int) -> Union[RawFd, ErrorId]: ...
def pathrs_inroot_resolve(rootfd: RawFd, path: CString) -> Union[RawFd, ErrorId]: ...
def pathrs_inroot_resolve_nofollow(
    rootfd: RawFd, path: CString
) -> Union[RawFd, ErrorId]: ...
def pathrs_inroot_open(
    rootfd: RawFd, path: CString, flags: int
) -> Union[RawFd, ErrorId]: ...
def pathrs_inroot_creat(
    rootfd: RawFd, path: CString, flags: int, filemode: int
) -> Union[RawFd, ErrorId]: ...
def pathrs_inroot_rename(
    rootfd: RawFd, src: CString, dst: CString, flags: int
) -> Union[Literal[0], ErrorId]: ...
def pathrs_inroot_rmdir(rootfd: RawFd, path: CString) -> Union[Literal[0], ErrorId]: ...
def pathrs_inroot_unlink(
    rootfd: RawFd, path: CString
) -> Union[Literal[0], ErrorId]: ...
def pathrs_inroot_remove_all(rootfd: RawFd, path: CString) -> Union[RawFd, ErrorId]: ...
def pathrs_inroot_mkdir(
    rootfd: RawFd, path: CString, mode: int
) -> Union[Literal[0], ErrorId]: ...
def pathrs_inroot_mkdir_all(
    rootfd: RawFd, path: CString, mode: int
) -> Union[Literal[0], ErrorId]: ...
def pathrs_inroot_mknod(
    rootfd: RawFd, path: CString, mode: int, dev: int
) -> Union[Literal[0], ErrorId]: ...
def pathrs_inroot_hardlink(
    rootfd: RawFd, path: CString, target: CString
) -> Union[Literal[0], ErrorId]: ...
def pathrs_inroot_symlink(
    rootfd: RawFd, path: CString, target: CString
) -> Union[Literal[0], ErrorId]: ...
def pathrs_inroot_readlink(
    rootfd: RawFd, path: CString, linkbuf: CBuffer, linkbuf_size: int
) -> Union[int, ErrorId]: ...
