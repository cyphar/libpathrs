#!/usr/bin/env python3
# libpathrs: safe path resolution on Linux
# Copyright (C) 2019-2024 Aleksa Sarai <cyphar@cyphar.com>
# Copyright (C) 2019-2024 SUSE LLC
#
# This program is free software: you can redistribute it and/or modify it under
# the terms of the GNU Lesser General Public License as published by the Free
# Software Foundation, either version 3 of the License, or (at your option) any
# later version.
#
# This program is distributed in the hope that it will be useful, but WITHOUT
# ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
# FOR A PARTICULAR PURPOSE. See the GNU General Public License for more
# details.
#
# You should have received a copy of the GNU Lesser General Public License
# along with this program. If not, see <https://www.gnu.org/licenses/>.

# File: examples/python/procfs.py
#
# An example program which does some basic procfs operations.

import os
import sys

sys.path.append(os.path.dirname(__file__) + "/../contrib/bindings/python")
from pathrs import procfs


def chomp(s):
    for nl in ["\r\n", "\r", "\n"]:
        if s.endswith(nl):
            return s[: -len(nl)]
    return s


# TODO: Pretty-print errors...


def base_str(args):
    base = args.procfs_base
    if base == procfs.PROC_ROOT:
        return "/proc"
    elif base == procfs.PROC_SELF:
        return "/proc/self"
    elif base == procfs.PROC_THREAD_SELF:
        return "/proc/thread-self"
    elif args._pid is not None:
        return f"/proc/{args._pid}"
    return "<unknown proc base>"


def do_open(args):
    base = args.procfs_base
    subpath = args.subpath

    # TODO: Support O_* flags.
    extra_flags = 0
    if not args.follow_trailing:
        extra_flags |= os.O_NOFOLLOW

    if args.debug:
        print(f"fullpath: {base_str(args)}/{subpath}")
        print("---")

    with procfs.open(base, subpath, extra_flags=extra_flags) as f:
        for line in f:
            print(chomp(line))


def do_readlink(args):
    base = args.procfs_base
    subpath = args.subpath

    if args.debug:
        print(f"fullpath: {base_str(args)}/{subpath}")
        print("---")

    link_target = procfs.readlink(base, subpath)
    print(link_target)


def main(*args):
    import argparse

    class ProcPidAction(argparse.Action):
        def __init__(self, option_strings, dest, nargs=None, **kwargs):
            if nargs is not None:
                raise ValueError("nargs not allowed")
            super().__init__(option_strings, dest, **kwargs)

        def __call__(self, parser, namespace, values, option_string=None):
            pid = int(values)
            setattr(namespace, "_pid", pid)
            setattr(namespace, self.dest, procfs.PROC_PID(pid))

    parser = argparse.ArgumentParser(prog="procfs.py")
    parser.add_argument("--debug", action="store_true", help="output debug messages")

    procfs_base = parser.add_argument_group(
        title="base path",
        description="Specify what base directory to use as the root of the procfs operation.",
    ).add_mutually_exclusive_group(required=True)

    procfs_base.add_argument(
        "-R",
        "--root",
        dest="procfs_base",
        action="store_const",
        const=procfs.PROC_ROOT,
        help="/proc",
    )
    procfs_base.add_argument(
        "-s",
        "--self",
        dest="procfs_base",
        action="store_const",
        const=procfs.PROC_SELF,
        help="/proc/self",
    )
    procfs_base.add_argument(
        "--thread-self",
        dest="procfs_base",
        action="store_const",
        const=procfs.PROC_THREAD_SELF,
        help="/proc/thread-self",
    )
    procfs_base.add_argument(
        "-p",
        "--pid",
        metavar="PID",
        dest="procfs_base",
        action=ProcPidAction,
        help="/proc/$PID (racey)",
    )
    procfs_base.set_defaults(_pid=None)

    subparser = parser.add_subparsers(help="procfs operations")

    # procfs.py readlink <subpath>
    readlink_cmd = subparser.add_parser("readlink", help="safe readlink")
    readlink_cmd.set_defaults(func=do_readlink)

    # procfs.py open <subpath>
    open_cmd = subparser.add_parser("open", help="open(O_NOFOLLOW)")
    open_cmd.add_argument(
        "--follow-trailing",
        dest="follow_trailing",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="whether to follow trailing symlinks when opening paths (this allows you to re-open magic-links)",
    )
    # TODO: Support O_* flags.
    open_cmd.set_defaults(func=do_open)

    parser.add_argument("subpath")

    args = parser.parse_args(args)
    if args.debug:
        print(f"procfs base: {args.procfs_base:#x} (pid={args._pid})")
        print(f"subcommand: {args.func}")
        print(f"subpath: {args.subpath}")
    args.func(args)


if __name__ == "__main__":
    main(*sys.argv[1:])
