# SPDX-License-Identifier: MPL-2.0
#
# libpathrs: safe path resolution on Linux
# Copyright (C) 2019-2025 Aleksa Sarai <cyphar@cyphar.com>
# Copyright (C) 2019-2025 SUSE LLC
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.


CC ?= gcc

CFLAGS := $(shell pkg-config --cflags pathrs)
LDFLAGS := $(shell pkg-config --libs-only-L --libs-only-other pathrs)
LDLIBS := $(shell pkg-config --libs-only-l pathrs)

.PHONY: all
all: $(patsubst %.c,%,$(wildcard *.c))

cat_multithread: LDLIBS += -lpthread

.PHONY: smoke-test
smoke-test: cat cat_multithread
	./cat . ../../cat.c >/dev/null
	./cat_multithread . ../../cat_multithread.c >/dev/null
