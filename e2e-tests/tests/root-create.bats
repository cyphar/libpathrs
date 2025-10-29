#!/usr/bin/bats -t
# SPDX-License-Identifier: MPL-2.0
#
# libpathrs: safe path resolution on Linux
# Copyright (C) 2019-2025 Aleksa Sarai <cyphar@cyphar.com>
# Copyright (C) 2019-2025 SUSE LLC
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

load helpers

function setup_file() {
	export ORIGINAL_UMASK="$(umask)"
}

function setup() {
	setup_tmpdirs
	umask 000
}

function teardown() {
	teardown_tmpdirs
	umask "$ORIGINAL_UMASK"
}

@test "root mkfile" {
	ROOT="$(setup_tmpdir)"

	pathrs-cmd root --root "$ROOT" mkfile --mode 0123 file
	[ "$status" -eq 0 ]
	grep -Fx "FILE-PATH $ROOT/file" <<<"$output"
	[ -e "$ROOT/file" ]
	[ -f "$ROOT/file" ]

	sane_run stat -c '%#a' "$ROOT/file"
	[[ "$output" == "0123" ]]
	sane_run stat -c '%F' "$ROOT/file"
	[[ "$output" == "regular empty file" ]]
	sane_run stat -c '%s' "$ROOT/file"
	[ "$output" -eq 0 ]
}

@test "root mkfile --oflags O_EXCL" {
	ROOT="$(setup_tmpdir)"

	echo "a random file" >"$ROOT/extant-file"
	[ "$(stat -c '%s' "$ROOT/extant-file")" -ne 0 ]
	chmod 0755 "$ROOT/extant-file"

	pathrs-cmd root --root "$ROOT" mkfile --mode 0123 extant-file
	[ "$status" -eq 0 ]
	grep -Fx "FILE-PATH $ROOT/extant-file" <<<"$output"
	[ -e "$ROOT/extant-file" ]
	[ -f "$ROOT/extant-file" ]

	sane_run stat -c '%#a' "$ROOT/extant-file"
	# The mode is not changed if the file already existed!
	[[ "$output" == "0755" ]]
	sane_run stat -c '%F' "$ROOT/extant-file"
	[[ "$output" == "regular file" ]]
	sane_run stat -c '%s' "$ROOT/extant-file"
	[ "$output" -ne 0 ]

	pathrs-cmd root --root "$ROOT" mkfile --oflags O_EXCL --mode 0123 extant-file
	check-errno EEXIST

	sane_run stat -c '%#a' "$ROOT/extant-file"
	# The mode is not changed if the file already existed!
	[[ "$output" == "0755" ]]
	sane_run stat -c '%F' "$ROOT/extant-file"
	[[ "$output" == "regular file" ]]
	sane_run stat -c '%s' "$ROOT/extant-file"
	[ "$output" -ne 0 ]

	# Make sure the file is not truncated if O_EXCL is violated.
	pathrs-cmd root --root "$ROOT" mkfile --oflags O_EXCL,O_TRUNC --mode 0123 extant-file
	check-errno EEXIST

	sane_run stat -c '%#a' "$ROOT/extant-file"
	# The mode is not changed if the file already existed!
	[[ "$output" == "0755" ]]
	sane_run stat -c '%F' "$ROOT/extant-file"
	[[ "$output" == "regular file" ]]
	sane_run stat -c '%s' "$ROOT/extant-file"
	[ "$output" -ne 0 ]
}

@test "root mkfile --oflags O_TRUNC" {
	ROOT="$(setup_tmpdir)"
	file="file-$RANDOM"

	echo "THIS SHOULD BE TRUNCATED" >"$ROOT/$file"
	[ "$(stat -c '%s' "$ROOT/$file")" -ne 0 ]
	chmod 0644 "$ROOT/$file"

	pathrs-cmd root --root "$ROOT" mkfile --oflags O_TRUNC --mode 0123 "$file"
	[ "$status" -eq 0 ]
	grep -Fx "FILE-PATH $ROOT/$file" <<<"$output"
	[ -e "$ROOT/$file" ]
	[ -f "$ROOT/$file" ]

	sane_run stat -c '%#a' "$ROOT/$file"
	# The mode is not changed if the file already existed!
	[[ "$output" == "0644" ]]
	sane_run stat -c '%F' "$ROOT/$file"
	[[ "$output" == "regular empty file" ]]
	sane_run stat -c '%s' "$ROOT/$file"
	[ "$output" -eq 0 ]
}

@test "root mkfile --oflags O_TMPFILE" {
	# FIXME FIXME FIXME
	skip "known bug <https://github.com/cyphar/libpathrs/issues/278>"

	ROOT="$(setup_tmpdir)"
	mkdir -p "$ROOT/var/tmp"

	pathrs-cmd root --root "$ROOT" mkfile --oflags O_TMPFILE --mode 0700 .
	[ "$status" -eq 0 ]
	# TODO: FILE-PATH?

	pathrs-cmd root --root "$ROOT" mkfile --oflags O_TMPFILE --mode 0700 /
	[ "$status" -eq 0 ]
	# TODO: FILE-PATH?

	pathrs-cmd root --root "$ROOT" mkfile --oflags O_TMPFILE --mode 0700 /var/tmp
	[ "$status" -eq 0 ]
	# TODO: FILE-PATH?
}

@test "root mkfile [non-existent parent component]" {
	ROOT="$(setup_tmpdir)"

	mkdir -p "$ROOT/foo/bar/baz"

	pathrs-cmd root --root "$ROOT" mkfile --mode 0123 foo/nope/baz
	check-errno ENOENT

	! [ -e "$ROOT/foo/nope" ]
	! [ -e "$ROOT/foo/nope/baz" ]
}

@test "root mkfile [bad parent component]" {
	ROOT="$(setup_tmpdir)"

	mkdir -p "$ROOT/foo"
	touch "$ROOT/foo/bar"

	pathrs-cmd root --root "$ROOT" mkfile --mode 0123 foo/bar/baz
	check-errno ENOTDIR

	[ -f "$ROOT/foo/bar" ]
	! [ -e "$ROOT/foo/bar/baz" ]
}

@test "root mkfile [umask]" {
	ROOT="$(setup_tmpdir)"

	umask 022
	pathrs-cmd root --root "$ROOT" mkfile --mode 0777 with-umask-022
	[ "$status" -eq 0 ]
	grep -Fx "FILE-PATH $ROOT/with-umask-022" <<<"$output"
	[ -f "$ROOT/with-umask-022" ]

	umask 027
	pathrs-cmd root --root "$ROOT" mkfile --mode 0777 with-umask-027
	grep -Fx "FILE-PATH $ROOT/with-umask-027" <<<"$output"
	[ "$status" -eq 0 ]
	[ -f "$ROOT/with-umask-027" ]

	umask 117
	pathrs-cmd root --root "$ROOT" mkfile --mode 0777 with-umask-117
	grep -Fx "FILE-PATH $ROOT/with-umask-117" <<<"$output"
	[ "$status" -eq 0 ]
	[ -f "$ROOT/with-umask-117" ]

	sane_run stat -c '%#a' "$ROOT/with-umask-022"
	[[ "$output" == "0755" ]]
	sane_run stat -c '%#a' "$ROOT/with-umask-027"
	[[ "$output" == "0750" ]]
	sane_run stat -c '%#a' "$ROOT/with-umask-117"
	[[ "$output" == "0660" ]]
}

@test "root mkfile [mode=7777]" {
	ROOT="$(setup_tmpdir)"

	pathrs-cmd root --root "$ROOT" mkfile --mode 07777 all-bits
	[ "$status" -eq 0 ]
	grep -Fx "FILE-PATH $ROOT/all-bits" <<<"$output"
	[ -f "$ROOT/all-bits" ]

	sane_run stat -c '%#a' "$ROOT/all-bits"
	[[ "$output" == "07777" ]]
}

@test "root mknod [file]" {
	ROOT="$(setup_tmpdir)"

	pathrs-cmd root --root "$ROOT" mknod --mode 0755 file f
	[ "$status" -eq 0 ]

	[ -e "$ROOT/file" ]
	[ -f "$ROOT/file" ]
	sane_run stat -c '%#a' "$ROOT/file"
	[[ "$output" == "0755" ]]
	sane_run stat -c '%F' "$ROOT/file"
	[[ "$output" == "regular empty file" ]]
	sane_run stat -c '%s' "$ROOT/file"
	[ "$output" -eq 0 ]
}

@test "root mkdir" {
	ROOT="$(setup_tmpdir)"

	pathrs-cmd root --root "$ROOT" mkdir --mode 0711 dir
	[ "$status" -eq 0 ]
	[ -e "$ROOT/dir" ]
	[ -d "$ROOT/dir" ]

	sane_run stat -c '%#a' "$ROOT/dir"
	[[ "$output" == "0711" ]]
	sane_run stat -c '%F' "$ROOT/dir"
	[[ "$output" == "directory" ]]
}

@test "root mkdir [non-existent parent component]" {
	ROOT="$(setup_tmpdir)"

	mkdir -p "$ROOT/foo/bar/baz"

	pathrs-cmd root --root "$ROOT" mkdir --mode 0123 foo/nope/baz
	check-errno ENOENT

	! [ -e "$ROOT/foo/nope" ]
	! [ -e "$ROOT/foo/nope/baz" ]
}

@test "root mkdir [bad parent component]" {
	ROOT="$(setup_tmpdir)"

	mkdir -p "$ROOT/foo"
	touch "$ROOT/foo/bar"

	pathrs-cmd root --root "$ROOT" mkdir --mode 0123 foo/bar/baz
	check-errno ENOTDIR

	[ -f "$ROOT/foo/bar" ]
	! [ -e "$ROOT/foo/bar/baz" ]
}

@test "root mkdir [umask]" {
	ROOT="$(setup_tmpdir)"

	umask 022
	pathrs-cmd root --root "$ROOT" mkdir --mode 0777 with-umask-022
	[ "$status" -eq 0 ]
	[ -d "$ROOT/with-umask-022" ]

	umask 027
	pathrs-cmd root --root "$ROOT" mkdir --mode 0777 with-umask-027
	[ "$status" -eq 0 ]
	[ -d "$ROOT/with-umask-027" ]

	umask 117
	pathrs-cmd root --root "$ROOT" mkdir --mode 0777 with-umask-117
	[ "$status" -eq 0 ]
	[ -d "$ROOT/with-umask-117" ]

	sane_run stat -c '%#a' "$ROOT/with-umask-022"
	[[ "$output" == "0755" ]]
	sane_run stat -c '%#a' "$ROOT/with-umask-027"
	[[ "$output" == "0750" ]]
	sane_run stat -c '%#a' "$ROOT/with-umask-117"
	[[ "$output" == "0660" ]]
}

@test "root mkdir [mode=7777]" {
	ROOT="$(setup_tmpdir)"

	pathrs-cmd root --root "$ROOT" mkdir --mode 07777 all-bits
	[ "$status" -eq 0 ]
	[ -d "$ROOT/all-bits" ]

	sane_run stat -c '%#a' "$ROOT/all-bits"
	# On Linux, mkdir(2) implicitly strips the setuid/setgid bits from mode.
	[[ "$output" == "01777" ]]
}

@test "root mknod [directory]" {
	ROOT="$(setup_tmpdir)"

	pathrs-cmd root --root "$ROOT" mknod --mode 0777 dir d
	[ "$status" -eq 0 ]

	[ -e "$ROOT/dir" ]
	[ -d "$ROOT/dir" ]
	sane_run stat -c '%#a' "$ROOT/dir"
	[[ "$output" == "0777" ]]
	sane_run stat -c '%F' "$ROOT/dir"
	[[ "$output" == "directory" ]]
}

@test "root mkdir-all" {
	ROOT="$(setup_tmpdir)"

	mkdir -p "$ROOT/some/random"
	chmod -R 0777 "$ROOT/some"

	pathrs-cmd root --root "$ROOT" mkdir-all --mode 0711 some/random/directory
	[ "$status" -eq 0 ]
	grep -Fx "HANDLE-PATH $ROOT/some/random/directory" <<<"$output"
	[ -d "$ROOT/some" ]
	[ -d "$ROOT/some/random" ]
	[ -d "$ROOT/some/random/directory" ]

	# Extant directories don't have their modes changed.
	sane_run stat -c '%#a' "$ROOT/some"
	[[ "$output" == "0777" ]]
	sane_run stat -c '%#a' "$ROOT/some/random"
	[[ "$output" == "0777" ]]
	# Only directories we created have their mode change.
	sane_run stat -c '%#a' "$ROOT/some/random/directory"
	[[ "$output" == "0711" ]]
	sane_run stat -c '%F' "$ROOT/some/random/directory"
	[[ "$output" == "directory" ]]
}

@test "root mkdir-all [non-existent parent component]" {
	ROOT="$(setup_tmpdir)"

	mkdir -p "$ROOT/foo/bar/baz"
	chmod -R 0777 "$ROOT/foo"

	pathrs-cmd root --root "$ROOT" mkdir-all --mode 0755 foo/nope/baz
	[ "$status" -eq 0 ]
	grep -Fx "HANDLE-PATH $ROOT/foo/nope/baz" <<<"$output"
	[ -d "$ROOT/foo/nope" ]
	[ -d "$ROOT/foo/nope/baz" ]

	# Extant directories don't have their modes changed.
	sane_run stat -c '%#a' "$ROOT/foo"
	[[ "$output" == "0777" ]]
	# Only directories we created have their mode change.
	sane_run stat -c '%#a' "$ROOT/foo/nope"
	[[ "$output" == "0755" ]]
	sane_run stat -c '%#a' "$ROOT/foo/nope/baz"
	[[ "$output" == "0755" ]]
	sane_run stat -c '%F' "$ROOT/foo/nope/baz"
	[[ "$output" == "directory" ]]
}

@test "root mkdir-all [bad parent component]" {
	ROOT="$(setup_tmpdir)"

	mkdir -p "$ROOT/foo"
	touch "$ROOT/foo/bar"

	pathrs-cmd root --root "$ROOT" mkdir-all --mode 0755 foo/bar/baz
	check-errno ENOTDIR

	[ -f "$ROOT/foo/bar" ]
	! [ -e "$ROOT/foo/bar/baz" ]
}

@test "root mkdir-all [umask]" {
	ROOT="$(setup_tmpdir)"

	umask 022
	pathrs-cmd root --root "$ROOT" mkdir-all --mode 0777 with-umask-022
	[ "$status" -eq 0 ]
	[ -d "$ROOT/with-umask-022" ]

	umask 027
	pathrs-cmd root --root "$ROOT" mkdir-all --mode 0777 with-umask-027
	[ "$status" -eq 0 ]
	[ -d "$ROOT/with-umask-027" ]

	umask 117
	pathrs-cmd root --root "$ROOT" mkdir-all --mode 0777 with-umask-117
	[ "$status" -eq 0 ]
	[ -d "$ROOT/with-umask-117" ]

	sane_run stat -c '%#a' "$ROOT/with-umask-022"
	[[ "$output" == "0755" ]]
	sane_run stat -c '%#a' "$ROOT/with-umask-027"
	[[ "$output" == "0750" ]]
	sane_run stat -c '%#a' "$ROOT/with-umask-117"
	[[ "$output" == "0660" ]]
}

@test "root mkdir-all [mode=7777]" {
	ROOT="$(setup_tmpdir)"

	pathrs-cmd root --root "$ROOT" mkdir-all --mode 07777 all-bits
	# FIXME: https://github.com/cyphar/libpathrs/issues/280
	check-errno EINVAL
	! [ -d "$ROOT/all-bits" ]

	pathrs-cmd root --root "$ROOT" mkdir-all --mode 01777 mode-1777
	[ "$status" -eq 0 ]
	! [ -d "$ROOT/mode-1777" ]

	sane_run stat -c '%#a' "$ROOT/mode-1777"
	[[ "$output" == "01777" ]]
}

@test "root mknod [whiteout]" {
	requires can-mkwhiteout

	ROOT="$(setup_tmpdir)"

	pathrs-cmd root --root "$ROOT" mknod --mode 0666 "c-0-0" c 0 0
	[ "$status" -eq 0 ]

	[ -e "$ROOT/c-0-0" ]
	[ -c "$ROOT/c-0-0" ]
	sane_run stat -c '%#a' "$ROOT/c-0-0"
	[[ "$output" == "0666" ]]
	sane_run stat -c '%F:%Hr:%Lr' "$ROOT/c-0-0"
	[[ "$output" == "character special file:0:0" ]]
}

@test "root mknod [fifo]" {
	ROOT="$(setup_tmpdir)"

	pathrs-cmd root --root "$ROOT" mknod --mode 0664 fifo p
	[ "$status" -eq 0 ]

	[ -e "$ROOT/fifo" ]
	[ -p "$ROOT/fifo" ]
	sane_run stat -c '%#a' "$ROOT/fifo"
	[[ "$output" == "0664" ]]
	sane_run stat -c '%F' "$ROOT/fifo"
	[[ "$output" == "fifo" ]]
}

@test "root mknod [char device]" {
	requires root

	ROOT="$(setup_tmpdir)"

	pathrs-cmd root --root "$ROOT" mknod --mode 0644 chr-111-222 c 111 222
	[ "$status" -eq 0 ]

	[ -e "$ROOT/chr-111-222" ]
	[ -c "$ROOT/chr-111-222" ]
	sane_run stat -c '%#a' "$ROOT/chr-111-222"
	[[ "$output" == "0644" ]]
	sane_run stat -c '%F:%Hr:%Lr' "$ROOT/chr-111-222"
	[[ "$output" == "character special file:111:222" ]]
}

@test "root mknod [block device]" {
	requires root

	ROOT="$(setup_tmpdir)"

	pathrs-cmd root --root "$ROOT" mknod --mode 0600 blk-123-456 b 123 456
	[ "$status" -eq 0 ]

	[ -e "$ROOT/blk-123-456" ]
	[ -b "$ROOT/blk-123-456" ]
	sane_run stat -c '%#a' "$ROOT/blk-123-456"
	[[ "$output" == "0600" ]]
	sane_run stat -c '%F:%Hr:%Lr' "$ROOT/blk-123-456"
	[[ "$output" == "block special file:123:456" ]]
}

@test "root mknod [non-existent parent component]" {
	requires can-mkwhiteout

	ROOT="$(setup_tmpdir)"

	mkdir -p "$ROOT/foo/bar/baz"

	pathrs-cmd root --root "$ROOT" mknod --mode 0123 foo/nope/baz c 0 0
	check-errno ENOENT

	! [ -e "$ROOT/foo/nope" ]
	! [ -e "$ROOT/foo/nope/baz" ]
}

@test "root mknod [bad parent component]" {
	requires can-mkwhiteout

	ROOT="$(setup_tmpdir)"

	mkdir -p "$ROOT/foo"
	touch "$ROOT/foo/bar"

	pathrs-cmd root --root "$ROOT" mknod --mode 0123 foo/bar/baz c 0 0
	check-errno ENOTDIR

	[ -f "$ROOT/foo/bar" ]
	! [ -e "$ROOT/foo/bar/baz" ]
}

@test "root mknod [umask]" {
	ROOT="$(setup_tmpdir)"

	umask 022
	pathrs-cmd root --root "$ROOT" mknod --mode 0777 with-umask-022 f
	[ "$status" -eq 0 ]
	[ -f "$ROOT/with-umask-022" ]

	umask 027
	pathrs-cmd root --root "$ROOT" mknod --mode 0777 with-umask-027 f
	[ "$status" -eq 0 ]
	[ -f "$ROOT/with-umask-027" ]

	umask 117
	pathrs-cmd root --root "$ROOT" mknod --mode 0777 with-umask-117 f
	[ "$status" -eq 0 ]
	[ -f "$ROOT/with-umask-117" ]

	sane_run stat -c '%#a' "$ROOT/with-umask-022"
	[[ "$output" == "0755" ]]
	sane_run stat -c '%#a' "$ROOT/with-umask-027"
	[[ "$output" == "0750" ]]
	sane_run stat -c '%#a' "$ROOT/with-umask-117"
	[[ "$output" == "0660" ]]
}

@test "root mknod [mode=7777]" {
	ROOT="$(setup_tmpdir)"

	pathrs-cmd root --root "$ROOT" mknod --mode 07777 all-bits d
	[ "$status" -eq 0 ]

	[ -d "$ROOT/all-bits" ]
	sane_run stat -c '%#a' "$ROOT/all-bits"
	# On Linux, mknod(2) implicitly strips the setuid/setgid bits from mode.
	[[ "$output" == "01777" ]]
}
