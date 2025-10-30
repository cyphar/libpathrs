#!/bin/bash
# SPDX-License-Identifier: MPL-2.0
#
# libpathrs: safe path resolution on Linux
# Copyright (C) 2019-2025 Aleksa Sarai <cyphar@cyphar.com>
# Copyright (C) 2019-2025 SUSE LLC
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

ifneq ($(RUN_AS),)
	SUDO := sudo -u $(RUN_AS)
endif

BATS ?= bats

ALL_TESTS := $(patsubst cmd/%/Makefile,test-%,$(wildcard cmd/*/Makefile))

.DEFAULT: test-all
.PHONY: test-all
test-all: $(ALL_TESTS)

test-%: cmd/%/pathrs-cmd FORCE
	$(SUDO) PATHRS_CMD=$< $(BATS) -t tests/

cmd/%/pathrs-cmd: FORCE
	make -C $(@D) $(@F)

# .PHONY doesn't work with patterns so we need to create a dummy rule.
# See <https://www.gnu.org/software/make/manual/html_node/Force-Targets.html>
# and <https://www.gnu.org/software/make/manual/html_node/Phony-Targets.html>.
FORCE:
