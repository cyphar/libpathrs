#!/usr/bin/bats -t
# SPDX-License-Identifier: MPL-2.0
#
# libpathrs: safe path resolution on Linux
# Copyright (C) 2019-2025 SUSE LLC
# Copyright (C) 2026 Aleksa Sarai <cyphar@cyphar.com>
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

load helpers

function setup() {
	setup_tmpdirs
}

function teardown() {
	teardown_tmpdirs
}

@test "root readlink" {
	ROOT="$(setup_tmpdir)"

	ln -s /some/random/path "$ROOT/link"

	pathrs-cmd root --root "$ROOT" readlink link
	[ "$status" -eq 0 ]
	grep -Fx 'LINK-TARGET /some/random/path' <<<"$output"
}

@test "root readlink [stacked trailing]" {
	ROOT="$(setup_tmpdir)"

	ln -s /some/random/path "$ROOT/link-a"
	ln -s /link-a "$ROOT/link-b"
	ln -s ../../link-b "$ROOT/link-c"

	pathrs-cmd root --root "$ROOT" readlink link-a
	[ "$status" -eq 0 ]
	grep -Fx 'LINK-TARGET /some/random/path' <<<"$output"

	pathrs-cmd root --root "$ROOT" readlink link-b
	[ "$status" -eq 0 ]
	grep -Fx 'LINK-TARGET /link-a' <<<"$output"

	pathrs-cmd root --root "$ROOT" readlink link-c
	[ "$status" -eq 0 ]
	grep -Fx 'LINK-TARGET ../../link-b' <<<"$output"
}

@test "root readlink [non-symlink]" {
	ROOT="$(setup_tmpdir)"

	echo "/some/random/path" >"$ROOT/file"

	pathrs-cmd root --root "$ROOT" readlink file
	check-errno ENOENT
	[[ "$output" == *"error:"*"readlinkat"* ]] # Make sure the error is from readlinkat(2).
}
