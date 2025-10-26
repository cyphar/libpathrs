#!/usr/bin/env python3
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
import sys
import stat
import argparse
from typing import Optional, Self, Sequence, Protocol, Tuple

sys.path.append(os.path.dirname(__file__) + "/../../../contrib/bindings/python")
import pathrs
from pathrs import procfs
from pathrs.procfs import ProcfsHandle, ProcfsBase


class FilenoFile(Protocol):
    def fileno(self) -> int: ...


def fdpath(fd: FilenoFile) -> str:
    return ProcfsHandle.cached().readlink(procfs.PROC_THREAD_SELF, f"fd/{fd.fileno()}")


def root_resolve(args: argparse.Namespace):
    root: pathrs.Root = args.root
    subpath: str = args.subpath
    follow: bool = args.follow
    reopen: Optional[int] = args.reopen

    with root.resolve(subpath, follow_trailing=follow) as handle:
        print("HANDLE-PATH", fdpath(handle))
        if reopen is not None:
            with handle.reopen_raw(reopen) as file:
                print("FILE-PATH", fdpath(file))


def root_open(args: argparse.Namespace):
    root: pathrs.Root = args.root
    subpath: str = args.subpath
    oflags: int = args.oflags
    follow: bool = args.follow

    if not follow:
        oflags |= os.O_NOFOLLOW

    with root.open_raw(subpath, oflags) as file:
        print("FILE-PATH", fdpath(file))


def root_mkfile(args: argparse.Namespace):
    root: pathrs.Root = args.root
    subpath: str = args.subpath
    oflags: int = args.oflags
    mode: int = args.mode

    with root.creat_raw(subpath, oflags, mode) as file:
        print("FILE-PATH", fdpath(file))


def root_mkdir(args: argparse.Namespace):
    root: pathrs.Root = args.root
    subpath: str = args.subpath
    mode: int = args.mode

    root.mkdir(subpath, mode)


def root_mkdir_all(args: argparse.Namespace):
    root: pathrs.Root = args.root
    subpath: str = args.subpath
    mode: int = args.mode

    with root.mkdir_all(subpath, mode) as handle:
        print("HANDLE-PATH", fdpath(handle))


def root_mknod(args: argparse.Namespace):
    root: pathrs.Root = args.root
    subpath: str = args.subpath
    mode: int = args.mode
    inode_type: int
    inode_dev: int
    (inode_type, inode_dev) = args.type

    mode |= inode_type
    root.mknod(subpath, mode, inode_dev)


def root_hardlink(args: argparse.Namespace):
    root: pathrs.Root = args.root
    target: str = args.target
    linkname: str = args.linkname

    # TODO: These arguments need to get swapped.
    root.hardlink(linkname, target)


def root_symlink(args: argparse.Namespace):
    root: pathrs.Root = args.root
    target: str = args.target
    linkname: str = args.linkname

    # TODO: These arguments need to get swapped.
    root.symlink(linkname, target)


def root_readlink(args: argparse.Namespace):
    root: pathrs.Root = args.root
    subpath: str = args.subpath

    target = root.readlink(subpath)
    print("LINK-TARGET", target)


def root_unlink(args: argparse.Namespace):
    root: pathrs.Root = args.root
    subpath: str = args.subpath

    root.unlink(subpath)


def root_rmdir(args: argparse.Namespace):
    root: pathrs.Root = args.root
    subpath: str = args.subpath

    root.rmdir(subpath)


def root_rmdir_all(args: argparse.Namespace):
    root: pathrs.Root = args.root
    subpath: str = args.subpath

    root.remove_all(subpath)


def root_rename(args: argparse.Namespace):
    root: pathrs.Root = args.root
    source: str = args.source
    destination: str = args.destination

    clobber: bool = args.clobber
    exchange: bool = args.exchange
    whiteout: bool = args.whiteout

    RENAME_NOREPLACE: int = 1 << 0
    RENAME_EXCHANGE: int = 1 << 1
    RENAME_WHITEOUT: int = 1 << 2

    rename_args: int = 0
    if not clobber:
        rename_args |= RENAME_NOREPLACE
    if exchange:
        rename_args |= RENAME_EXCHANGE
    if whiteout:
        rename_args |= RENAME_WHITEOUT

    root.rename(source, destination, rename_args)


def procfs_open(args: argparse.Namespace):
    unmasked: bool = args.unmasked
    proc = ProcfsHandle.new(unmasked=unmasked)

    base: ProcfsBase = args.procfs_base
    subpath: str = args.subpath
    oflags: int = args.oflags
    follow: bool = args.follow

    if not follow:
        oflags |= os.O_NOFOLLOW

    with proc.open_raw(base, subpath, oflags) as file:
        print("FILE-PATH", fdpath(file))


def procfs_readlink(args: argparse.Namespace):
    unmasked: bool = args.unmasked
    proc = ProcfsHandle.new(unmasked=unmasked)

    base: ProcfsBase = args.procfs_base
    subpath: str = args.subpath

    target = proc.readlink(base, subpath)
    print("LINK-TARGET", target)


def parse_args(
    args: tuple[str, ...],
) -> Tuple[argparse.ArgumentParser, argparse.Namespace]:
    parser = argparse.ArgumentParser(prog="pathrs-cmd")
    parser.set_defaults(func=None)
    top_subparser = parser.add_subparsers()

    def add_mode_flag(
        parser: argparse.ArgumentParser,
        name: str,
        default: int = 0o644,
        required: bool = False,
        help: Optional[str] = None,
    ) -> None:
        parser.add_argument(
            f"--{name}",
            type=lambda mode: int(mode, 8),
            default=default,
            required=required,
            help=help,
        )

    # TODO: Should this be a class?
    def parse_oflags(flags: str) -> int:
        VALID_FLAGS = {
            # Access modes (including O_PATH).
            "O_RDWR": os.O_RDWR,
            "O_RDONLY": os.O_RDONLY,
            "O_WRONLY": os.O_WRONLY,
            "O_PATH": os.O_PATH,
            # Fd flags.
            "O_CLOEXEC": os.O_CLOEXEC,
            # Control lookups.
            "O_NOFOLLOW": os.O_NOFOLLOW,
            "O_DIRECTORY": os.O_DIRECTORY,
            "O_NOCTTY": os.O_NOCTTY,
            # NOTE: This flag contains O_DIRECTORY!
            "O_TMPFILE": os.O_TMPFILE,
            # File creation.
            "O_CREAT": os.O_CREAT,
            "O_EXCL": os.O_EXCL,
            "O_TRUNC": os.O_TRUNC,
            "O_APPEND": os.O_APPEND,
            # Sync.
            "O_SYNC": os.O_SYNC,
            "O_ASYNC": os.O_ASYNC,
            "O_DSYNC": os.O_DSYNC,
            "O_FSYNC": os.O_FSYNC,
            "O_RSYNC": os.O_RSYNC,
            "O_DIRECT": os.O_DIRECT,
            "O_NDELAY": os.O_NDELAY,
            "O_NOATIME": os.O_NOATIME,
            "O_NONBLOCK": os.O_NONBLOCK,
        }

        all_flags = 0
        for flag in flags.replace(",", "|").split("|"):
            flag = flag.upper()
            if not flag.startswith("O_"):
                flag = "O_" + flag
            try:
                all_flags |= VALID_FLAGS[flag]
            except KeyError:
                raise ValueError(f"flag {flag:r} is not a valid O_* flag")
        return all_flags

    def add_o_flag(
        parser: argparse.ArgumentParser,
        name: str,
        default: Optional[int] = os.O_RDONLY,
        required: bool = False,
        help: Optional[str] = None,
    ) -> None:
        parser.add_argument(
            f"--{name}",
            metavar="O_*",
            type=parse_oflags,
            default=default,
            required=required,
            help=f"{help or name} (comma- or |-separated)",
        )

    # root --root <root> ... commands
    root_parser = top_subparser.add_parser("root", help="Root::* operations")
    root_parser.add_argument(
        "--root", type=pathrs.Root, required=True, help="root path"
    )
    root_subparser = root_parser.add_subparsers()

    # root resolve [--reopen <flags>] [--[no-]follow] subpath
    root_resolve_parser = root_subparser.add_parser(
        "resolve", help="resolve a path inside the root"
    )
    root_resolve_parser.add_argument(
        "--follow",
        default=True,
        action=argparse.BooleanOptionalAction,
        help="follow trailing symlinks",
    )
    add_o_flag(
        root_resolve_parser,
        "reopen",
        default=None,
        help="reopen the handle with these O_* flags",
    )
    root_resolve_parser.add_argument("subpath", help="path inside the root")
    root_resolve_parser.set_defaults(func=root_resolve)
    del root_resolve_parser

    # root open [--oflags <oflags>] [--[no-]follow] subpath
    root_open_parser = root_subparser.add_parser(
        "open", help="open a path inside the root"
    )
    root_open_parser.add_argument(
        "--follow",
        default=True,
        action=argparse.BooleanOptionalAction,
        help="follow trailing symlinks",
    )
    add_o_flag(
        root_open_parser, "oflags", help="O_* flags to use when opening the file"
    )
    root_open_parser.add_argument("subpath", help="path inside the root")
    root_open_parser.set_defaults(func=root_open)
    del root_open_parser

    # root mkfile [--oflags <oflags>] [--mode <mode>] subpath
    root_mkfile_parser = root_subparser.add_parser(
        "mkfile", help="make an empty file inside the root"
    )
    add_o_flag(
        root_mkfile_parser, "oflags", help="O_* flags to use when creating the file"
    )
    add_mode_flag(root_mkfile_parser, "mode", help="created file mode")
    root_mkfile_parser.add_argument("subpath", help="path inside the root")
    root_mkfile_parser.set_defaults(func=root_mkfile)
    del root_mkfile_parser

    # root mkdir [--mode <mode>] subpath
    root_mkdir_parser = root_subparser.add_parser(
        "mkdir", help="make an empty directory inside the root"
    )
    add_mode_flag(
        root_mkdir_parser,
        "mode",
        default=0o755,
        help="file mode for the created directory",
    )
    root_mkdir_parser.add_argument("subpath", help="path inside the root")
    root_mkdir_parser.set_defaults(func=root_mkdir)
    del root_mkdir_parser

    # root mkdir-all [--mode <mode>] subpath
    root_mkdir_all_parser = root_subparser.add_parser(
        "mkdir-all",
        help="make a directory (including parents) inside the root",
    )
    add_mode_flag(
        root_mkdir_all_parser,
        "mode",
        default=0o755,
        help="file mode for the created directories",
    )
    root_mkdir_all_parser.add_argument("subpath", help="path inside the root")
    root_mkdir_all_parser.set_defaults(func=root_mkdir_all)
    del root_mkdir_all_parser

    # root mknod [--mode <mode>] <subpath> <type> [<major> <minor>]
    root_mknod_parser = root_subparser.add_parser(
        "mknod", help="make an inode inside the root"
    )
    add_mode_flag(root_mknod_parser, "mode", help="file mode for the new inode")
    root_mknod_parser.add_argument("subpath", help="path inside the root")

    class MknodTypeAction(argparse.Action):
        def __call__(
            self: Self,
            parser: argparse.ArgumentParser,
            namespace: argparse.Namespace,
            values: Optional[str | Sequence[str]],
            option_string: Optional[str] = None,
        ):
            inode_type: str
            dev: int
            match values:
                case (inode_type, major, minor):
                    dev = os.makedev(int(major), int(minor))
                case (inode_type,):
                    dev = 0
                case _:
                    raise ValueError(f"invalid mknod type {values}")
            try:
                inode_ftype = {
                    "f": stat.S_IFREG,
                    "d": stat.S_IFDIR,
                    "p": stat.S_IFIFO,
                    "b": stat.S_IFBLK,
                    "c": stat.S_IFCHR,
                    "u": stat.S_IFCHR,  # alias for "c"
                }[inode_type]
            except KeyError:
                raise ValueError(f"invalid mknod type {values}")
            setattr(namespace, self.dest, (inode_ftype, dev))

    root_mknod_parser.add_argument(
        "type",
        nargs=argparse.PARSER,
        action=MknodTypeAction,
        help="inode type to create (like mknod(1))",
    )
    root_mknod_parser.set_defaults(func=root_mknod)
    del root_mknod_parser

    # root hardlink <target> <linkname>
    root_hardlink_parser = root_subparser.add_parser(
        "hardlink", help="make a hardlink inside the root"
    )
    root_hardlink_parser.add_argument(
        "target",
        help="target path of the hardlink inside the root (must already exist)",
    )
    root_hardlink_parser.add_argument(
        "linkname", help="path inside the root for the new hardlink"
    )
    root_hardlink_parser.set_defaults(func=root_hardlink)
    del root_hardlink_parser

    # root symlink <target> <linkname>
    root_symlink_parser = root_subparser.add_parser(
        "symlink", help="make a symbolic link inside the root"
    )
    root_symlink_parser.add_argument("target", help="target path of the symlink")
    root_symlink_parser.add_argument(
        "linkname", help="path inside the root for the new symlink"
    )
    root_symlink_parser.set_defaults(func=root_symlink)
    del root_symlink_parser

    # root readlink <subpath>
    root_readlink_parser = root_subparser.add_parser(
        "readlink", help="read the target path of a symbolic link inside the root"
    )
    root_readlink_parser.add_argument("subpath", help="path inside the root")
    root_readlink_parser.set_defaults(func=root_readlink)
    del root_readlink_parser

    # root unlink <subpath>
    root_unlink_parser = root_subparser.add_parser(
        "unlink", help="remove a file inside the root"
    )
    root_unlink_parser.add_argument("subpath", help="path inside the root")
    root_unlink_parser.set_defaults(func=root_unlink)
    del root_unlink_parser

    # root rmdir <subpath>
    root_rmdir_parser = root_subparser.add_parser(
        "rmdir", help="remove an (empty) directory inside the root"
    )
    root_rmdir_parser.add_argument("subpath", help="path inside the root")
    root_rmdir_parser.set_defaults(func=root_rmdir)
    del root_rmdir_parser

    # root rmdir-all <subpath>
    root_rmdir_all_parser = root_subparser.add_parser(
        "rmdir-all", help="remove a path (recursively) inside the root"
    )
    root_rmdir_all_parser.add_argument("subpath", help="path inside the root")
    root_rmdir_all_parser.set_defaults(func=root_rmdir_all)
    del root_rmdir_all_parser

    # root rename [--exchange] [--whiteout] [--[no-]clobber] <src> <dst>
    root_rename_parser = root_subparser.add_parser(
        "rename", help="rename a path inside the root"
    )
    root_rename_parser.add_argument(
        "--whiteout",
        action="store_true",
        help="create whiteout inode in place of source",
    )
    root_rename_parser.add_argument(
        "--exchange",
        action="store_true",
        help="swap source and destination inodes",
    )
    root_rename_parser.add_argument(
        "--clobber",
        default=True,
        action=argparse.BooleanOptionalAction,
        help="allow rename target to be clobbered",
    )
    root_rename_parser.add_argument("source", help="source path inside the root")
    root_rename_parser.add_argument(
        "destination", help="destination path inside the root"
    )
    root_rename_parser.set_defaults(func=root_rename)
    del root_rename_parser

    del root_subparser, root_parser

    def parse_procfs_base(base: str) -> ProcfsBase:
        if base.startswith("pid="):
            pid = int(base.removeprefix("pid="))
            return procfs.PROC_PID(pid)
        else:
            match base:
                case "root":
                    return procfs.PROC_ROOT
                case "self":
                    return procfs.PROC_SELF
                case "thread-self":
                    return procfs.PROC_THREAD_SELF
                case _:
                    raise ValueError(f"invalid procfs base {base:r}")

    # procfs [--unmasked] --base <base> ... commands
    procfs_parser = top_subparser.add_parser(
        "procfs", help="ProcfsHandle::* operations"
    )
    procfs_parser.add_argument(
        "--unmasked", action="store_true", help="use unmasked procfs handle"
    )
    procfs_parser.add_argument(
        "--base",
        dest="procfs_base",
        type=parse_procfs_base,
        metavar="PROC_*",
        default=procfs.PROC_ROOT,
        help="base path for procfs operations (root, pid=<n>, self, thread-self)",
    )
    procfs_subparser = procfs_parser.add_subparsers()

    # procfs open [--oflags <oflags>] [--[no-]follow] subpath
    procfs_open_parser = procfs_subparser.add_parser(
        "open", help="open a subpath in procfs"
    )
    add_o_flag(
        procfs_open_parser, "oflags", help="O_* flags to use when opening the file"
    )
    procfs_open_parser.add_argument(
        "--follow",
        default=True,
        action=argparse.BooleanOptionalAction,
        help="follow trailing symlinks",
    )
    procfs_open_parser.add_argument("subpath", help="path inside procfs base")
    procfs_open_parser.set_defaults(func=procfs_open)
    del procfs_open_parser

    # procfs readlink <subpath>
    procfs_readlink_parser = procfs_subparser.add_parser(
        "readlink", help="read the target path of a symbolic link in procfs"
    )
    procfs_readlink_parser.add_argument("subpath", help="path inside procfs base")
    procfs_readlink_parser.set_defaults(func=procfs_readlink)
    del procfs_readlink_parser

    del procfs_subparser, procfs_parser

    return parser, parser.parse_args(args)


def main(*argv: str):
    parser, args = parse_args(argv)
    if args.func is not None:
        args.func(args)
    else:
        # Default to help page if no subcommand was selected.
        parser.print_help()


if __name__ == "__main__":
    try:
        main(*sys.argv[1:])
    except pathrs.PathrsError as e:
        print(f"ERRNO {e.errno} ({os.strerror(e.errno)})")
        print(f"error: {e.message}")
        sys.exit(1)
