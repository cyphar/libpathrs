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

# File: examples/python/sysctl.py
#
# An example program which does sysctl operations using the libpathrs safe
# procfs API.

import os
import sys

sys.path.append(os.path.dirname(__file__) + "/../contrib/bindings/python")
from pathrs import procfs
from pathrs.procfs import ProcfsHandle


def bail(*args):
    print("[!]", *args)
    os.exit(1)


def chomp(s: str) -> str:
    for nl in ["\r\n", "\r", "\n"]:
        if s.endswith(nl):
            return s[: -len(nl)]
    return s


def sysctl_subpath(name: str) -> str:
    # "kernel.foo.bar" -> /proc/sys/kernel/foo/bar
    return "sys/" + name.replace(".", "/")


PROCFS = ProcfsHandle.new(unmasked=True)


def sysctl_write(name: str, value: str) -> None:
    subpath = sysctl_subpath(name)
    with PROCFS.open(procfs.PROC_ROOT, subpath, "w") as f:
        f.write(value)


def sysctl_read(name: str, *, value_only: bool = False) -> None:
    subpath = sysctl_subpath(name)
    with PROCFS.open(procfs.PROC_ROOT, subpath, "r") as f:
        value = chomp(f.read())
        if value_only:
            print(f"{value}")
        else:
            print(f"{name} = {value}")


def main(*args):
    import argparse

    parser = argparse.ArgumentParser(
        prog="sysctl.py",
        description="A minimal implementation of sysctl(8) but using the libpathrs procfs API.",
    )
    parser.add_argument(
        "-n",
        "--values",
        dest="value_only",
        action="store_true",
        help="print only values of the given variable(s)",
    )
    parser.add_argument(
        "-w",
        "--write",
        action="store_true",
        help="enable writing a value to a variable",
    )
    parser.add_argument(
        "sysctls",
        nargs="*",
        metavar="variable[=value]",
        help="sysctl variable name (such as 'kernel.overflowuid')",
    )

    args = parser.parse_args(args)

    for sysctl in args.sysctls:
        if "=" in sysctl:
            if not args.write:
                bail("you must pass -w to enable sysctl writing")
            name, value = sysctl.split("=", maxsplit=1)
            sysctl_write(name, value)
        else:
            sysctl_read(sysctl, value_only=args.value_only)


if __name__ == "__main__":
    main(*sys.argv[1:])
