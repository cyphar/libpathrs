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

# File: examples/python/cat.py
#
# An example program which opens a file inside a root and outputs its contents
# using libpathrs.

import os
import sys

sys.path.append(os.path.dirname(__file__) + "/../contrib/bindings/python")
import pathrs


def chomp(s):
    for nl in ["\r\n", "\r", "\n"]:
        if s.endswith(nl):
            return s[: -len(nl)]
    return s


def main(root_path, unsafe_path):
    # Test that context managers work properly with WrappedFd:
    with pathrs.Root(root_path) as root:
        with root.open(unsafe_path, "r") as f:
            for line in f:
                line = chomp(line)
                print(line)


if __name__ == "__main__":
    main(*sys.argv[1:])
