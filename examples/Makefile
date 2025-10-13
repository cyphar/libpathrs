# SPDX-License-Identifier: MPL-2.0
#
# libpathrs: safe path resolution on Linux
# Copyright (C) 2019-2025 Aleksa Sarai <cyphar@cyphar.com>
# Copyright (C) 2019-2025 SUSE LLC
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.


.PHONY: smoke-test
smoke-test:
	make -C c $@
	make -C go $@
	make -C python $@
	make smoke-test-rust

.PHONY: smoke-test-rust
smoke-test-rust:
	make -C rust-cat smoke-test
