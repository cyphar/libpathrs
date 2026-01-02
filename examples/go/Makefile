# SPDX-License-Identifier: MPL-2.0
#
# libpathrs: safe path resolution on Linux
# Copyright (C) 2019-2025 SUSE LLC
# Copyright (C) 2026 Aleksa Sarai <cyphar@cyphar.com>
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

GO ?= go

GOMOD_FILES := go.mod go.sum

%: %.go $(GOMOD_FILES)
	$(GO) build -o $@ $<

.PHONY: smoke-test
smoke-test: cat sysctl
	./cat . ../../cat.go &>/dev/null
	./sysctl kernel.hostname kernel.overflowuid kernel.overflowgid
