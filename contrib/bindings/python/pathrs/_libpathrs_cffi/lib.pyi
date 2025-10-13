# libpathrs: safe path resolution on Linux
# Copyright (C) 2019-2025 Aleksa Sarai <cyphar@cyphar.com>
# Copyright (C) 2019-2025 SUSE LLC
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

from typing import type_check_only, Union

# TODO: Remove this once we only support Python >= 3.10.
from typing_extensions import TypeAlias, Literal

from .._pathrs import CBuffer, CString, ProcfsBase

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
