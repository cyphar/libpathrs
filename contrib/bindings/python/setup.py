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

import setuptools

from typing import Any, Dict


# This is only needed for backwards compatibility with older versions.
def parse_pyproject() -> Dict[str, Any]:
    try:
        import tomllib

        openmode = "rb"
    except ImportError:
        # TODO: Remove this once we only support Python >= 3.11.
        import toml as tomllib  # type: ignore

        openmode = "r"

    with open("pyproject.toml", openmode) as f:
        return tomllib.load(f)


pyproject = parse_pyproject()

setuptools.setup(
    # For backwards-compatibility with pre-pyproject setuptools.
    name=pyproject["project"]["name"],
    version=pyproject["project"]["version"],
    install_requires=pyproject["project"]["dependencies"],
    # Configure cffi building.
    ext_package="pathrs",
    platforms=["Linux"],
    cffi_modules=["pathrs/pathrs_build.py:ffibuilder"],
)
