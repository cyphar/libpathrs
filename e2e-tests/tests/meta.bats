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

# These are mainly meta-tests that ensure that pathrs-cmd supports certain
# specific features.

@test "pathrs-cmd [bad O_* flags]" {
	ROOT="$(setup_tmpdir)"

	touch "$ROOT/file"

	pathrs-cmd root --root "$ROOT" resolve --reopen O_BADFLAG file
	[ "$status" -ne 0 ]
	grep -F "O_BADFLAG" <<<"$output" # the error should mention the flag

	pathrs-cmd root --root "$ROOT" resolve --reopen badflag file
	[ "$status" -ne 0 ]
	grep -Fi "badflag" <<<"$output" # the error should mention the flag

	pathrs-cmd root --root "$ROOT" open --oflags O_BADFLAG file
	[ "$status" -ne 0 ]
	grep -F "O_BADFLAG" <<<"$output" # the error should mention the flag

	pathrs-cmd root --root "$ROOT" open --oflags badflag file
	[ "$status" -ne 0 ]
	grep -Fi "badflag" <<<"$output" # the error should mention the flag

	pathrs-cmd root --root "$ROOT" open --oflags rdonly,O_BADFLAG file
	[ "$status" -ne 0 ]
	grep -F "O_BADFLAG" <<<"$output" # the error should mention the flag

	pathrs-cmd root --root "$ROOT" open --oflags O_RDONLY,badflag file
	[ "$status" -ne 0 ]
	grep -Fi "badflag" <<<"$output" # the error should mention the flag
}

@test "pathrs-cmd [funky O_* flags]" {
	ROOT="$(setup_tmpdir)"

	echo "THIS SHOULD BE TRUNCATED" >"$ROOT/file"
	[ "$(stat -c '%s' "$ROOT/file")" -ne 0 ]
	chmod 0644 "$ROOT/file"

	pathrs-cmd root --root "$ROOT" open --oflags trunc file
	[ "$status" -eq 0 ]
	grep -Fx "FILE-PATH $ROOT/file" <<<"$output"

	sane_run stat -c '%F' "$ROOT/file"
	[[ "$output" == "regular empty file" ]]
	sane_run stat -c '%s' "$ROOT/file"
	[ "$output" -eq 0 ]
}

@test "pathrs-cmd mknod [bad type]" {
	ROOT="$(setup_tmpdir)"

	pathrs-cmd root --root "$ROOT" mknod file l
	[ "$status" -ne 0 ]
}
