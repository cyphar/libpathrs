# SPDX-License-Identifier: MPL-2.0
#
# libpathrs: safe path resolution on Linux
# Copyright (C) 2019-2025 Aleksa Sarai <cyphar@cyphar.com>
# Copyright (C) 2019-2025 SUSE LLC
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

PYTHON ?= python3
PIP ?= pip3

SRC_FILES := $(wildcard *.py pathrs/*.py)

dist: $(SRC_FILES) pyproject.toml
	$(PYTHON) -m build

.PHONY: clean
clean:
	rm -rf dist/ pathrs.*-info/ pathrs/__pycache__/ pathrs/_libpathrs_cffi.*

.PHONY: lint
lint:
	ruff format --check --diff .
	ruff check .
	mypy .

.PHONY: install
install: dist
	$(PIP) install dist/pathrs*.whl
