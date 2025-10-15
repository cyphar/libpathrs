# SPDX-License-Identifier: MPL-2.0
#
# libpathrs: safe path resolution on Linux
# Copyright (C) 2019-2025 Aleksa Sarai <cyphar@cyphar.com>
# Copyright (C) 2019-2025 SUSE LLC
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

CARGO ?= cargo

EXAMPLE_NAME := $(basename $(shell basename $$PWD))

.PHONY: build
build:
	$(CARGO) build --example $(EXAMPLE_NAME)

.PHONY: smoke-test
smoke-test: build
	../../target/debug/examples/$(EXAMPLE_NAME) . ../../main.rs &>/dev/null
