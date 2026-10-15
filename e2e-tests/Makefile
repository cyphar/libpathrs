#!/bin/bash
# SPDX-License-Identifier: MPL-2.0
#
# libpathrs: safe path resolution on Linux
# Copyright (C) 2019-2025 SUSE LLC
# Copyright (C) 2026 Aleksa Sarai <cyphar@cyphar.com>
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

ifneq ($(RUN_AS),)
	SUDO := sudo -u $(RUN_AS)
endif

BATS ?= bats

SRC_ROOT := $(abspath ..)

# By default we build all bindings' pathrs-cmds using the in-tree libpathrs.so.
# To test the bindings with the host libpathrs.so (for the pathrs-cmd
# implementations that support it -- i.e., the non-Rust ones) use
# PATHRS_E2E_USE_HOST_SO=1.
PATHRS_E2E_USE_HOST_SO ?=
ifeq ($(PATHRS_E2E_USE_HOST_SO),)
	PATHRS_E2E_PREFIX := $(SRC_ROOT)/target/e2e-libpathrs
	PATHRS_E2E_LIBDIR := $(PATHRS_E2E_PREFIX)/lib
	PATHRS_E2E_DEPS := libpathrs-dev

	# go: configure pkg-config to find out pathrs.pc.
	export PKG_CONFIG_PATH := $(PATHRS_E2E_LIBDIR)/pkgconfig$(if $(PKG_CONFIG_PATH),:$(PKG_CONFIG_PATH))
	# python: use our pathrs.h for cffi.
	export PATHRS_SRC_ROOT := $(SRC_ROOT)
	# c/go/python: set rpath so tests are guaranteed to use our libpathrs.so
	# rather than the host one (LD_LIBRARY_PATH is stripped by sudo).
	export LDFLAGS += -L$(PATHRS_E2E_LIBDIR) -Wl,-rpath,$(PATHRS_E2E_LIBDIR)
	export CGO_LDFLAGS += -Wl,-rpath,$(PATHRS_E2E_LIBDIR)
endif

ALL_TESTS := $(patsubst cmd/%/Makefile,test-%,$(wildcard cmd/*/Makefile))

.DEFAULT: test-all
.PHONY: test-all
test-all: $(ALL_TESTS)

test-%: cmd/%/pathrs-cmd FORCE
	$(SUDO) PATHRS_CMD=$< $(BATS) -t tests/

cmd/%/pathrs-cmd: FORCE
	make -C $(@D) $(@F)

# All bindings other than Rust link against libpathrs.so.
SO_PATHRS_CMDS := $(filter-out cmd/rust/pathrs-cmd,$(patsubst %/Makefile,%/pathrs-cmd,$(wildcard cmd/*/Makefile)))
$(SO_PATHRS_CMDS): $(PATHRS_E2E_DEPS)

.PHONY: libpathrs-dev
libpathrs-dev:
	make -C $(SRC_ROOT) release
	$(SRC_ROOT)/install.sh --disable-static \
		--prefix=$(PATHRS_E2E_PREFIX) --libdir=$(PATHRS_E2E_LIBDIR)

# .PHONY doesn't work with patterns so we need to create a dummy rule.
# See <https://www.gnu.org/software/make/manual/html_node/Force-Targets.html>
# and <https://www.gnu.org/software/make/manual/html_node/Phony-Targets.html>.
FORCE:
