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

import importlib
import importlib.metadata

from . import _pathrs
from ._pathrs import *  # noqa: F403 # We just re-export everything.

# In order get pydoc to include the documentation for the re-exported code from
# _pathrs, we need to include all of the members in __all__. Rather than
# duplicating the member list here explicitly, just re-export __all__.
__all__ = []
__all__ += _pathrs.__all__  # pyright doesn't support "=" here.

try:
    # In order to avoid drift between this version and the dist-info/ version
    # information, just fill __version__ with the dist-info/ information.
    __version__ = importlib.metadata.version("pathrs")
except importlib.metadata.PackageNotFoundError:
    # We're being run from a local directory without an installed version of
    # pathrs, so just fill in a dummy version.
    __version__ = "<unknown>"
