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

# TODO: All of these tests (especially for --reopen) are very limited because
# we cannot verify anything useful about the opened files. Ideally we would
# instead output something simple like the fdinfo, stat, and/or contents to
# verify against.

@test "root resolve [file]" {
	ROOT="$(setup_tmpdir)"

	mkdir -p "$ROOT/etc"
	echo "dummy passwd" >"$ROOT/etc/passwd"
	ln -s /../../../../../../../../etc "$ROOT/bad-passwd"

	pathrs-cmd root --root "$ROOT" resolve /etc/passwd
	[ "$status" -eq 0 ]
	grep -Fx "HANDLE-PATH $ROOT/etc/passwd" <<<"$output"
	! grep '^FILE-PATH' <<<"$output"

	pathrs-cmd root --root "$ROOT" resolve bad-passwd/passwd
	[ "$status" -eq 0 ]
	grep -Fx "HANDLE-PATH $ROOT/etc/passwd" <<<"$output"
	! grep '^FILE-PATH' <<<"$output"
}

@test "root resolve --reopen O_RDONLY [file]" {
	ROOT="$(setup_tmpdir)"

	mkdir -p "$ROOT/etc"
	echo "dummy passwd" >"$ROOT/etc/passwd"
	ln -s /../../../../../../../../etc "$ROOT/bad-passwd"

	pathrs-cmd root --root "$ROOT" resolve --reopen O_RDONLY /etc/passwd
	[ "$status" -eq 0 ]
	grep -Fx "HANDLE-PATH $ROOT/etc/passwd" <<<"$output"
	grep -Fx "FILE-PATH $ROOT/etc/passwd" <<<"$output"

	pathrs-cmd root --root "$ROOT" resolve --reopen O_RDONLY bad-passwd/passwd
	[ "$status" -eq 0 ]
	grep -Fx "HANDLE-PATH $ROOT/etc/passwd" <<<"$output"
	grep -Fx "FILE-PATH $ROOT/etc/passwd" <<<"$output"
}

@test "root open --oflags O_RDONLY [file]" {
	ROOT="$(setup_tmpdir)"

	mkdir -p "$ROOT/etc"
	echo "dummy passwd" >"$ROOT/etc/passwd"
	ln -s /../../../../../../../../etc "$ROOT/bad-passwd"

	pathrs-cmd root --root "$ROOT" open --oflags O_RDONLY /etc/passwd
	[ "$status" -eq 0 ]
	! grep -Fx "HANDLE-PATH $ROOT/etc/passwd" <<<"$output"
	grep -Fx "FILE-PATH $ROOT/etc/passwd" <<<"$output"

	pathrs-cmd root --root "$ROOT" open --oflags O_RDONLY bad-passwd/passwd
	[ "$status" -eq 0 ]
	! grep -Fx "HANDLE-PATH $ROOT/etc/passwd" <<<"$output"
	grep -Fx "FILE-PATH $ROOT/etc/passwd" <<<"$output"
}

@test "root resolve --reopen O_DIRECTORY [file]" {
	ROOT="$(setup_tmpdir)"

	mkdir -p "$ROOT/etc"
	echo "dummy passwd" >"$ROOT/etc/passwd"

	pathrs-cmd root --root "$ROOT" resolve --reopen O_DIRECTORY /etc/passwd
	check-errno ENOTDIR
}

@test "root open --oflags O_DIRECTORY [file]" {
	ROOT="$(setup_tmpdir)"

	mkdir -p "$ROOT/etc"
	echo "dummy passwd" >"$ROOT/etc/passwd"

	pathrs-cmd root --root "$ROOT" open --oflags O_DIRECTORY /etc/passwd
	check-errno ENOTDIR
}

@test "root resolve --reopen O_RDWR|O_TRUNC [file]" {
	ROOT="$(setup_tmpdir)"

	echo "THIS SHOULD BE TRUNCATED" >"$ROOT/to-trunc"
	[ "$(stat -c '%s' "$ROOT/to-trunc")" -ne 0 ]

	pathrs-cmd root --root "$ROOT" resolve --reopen O_RDWR,O_TRUNC to-trunc
	[ "$status" -eq 0 ]
	grep -Fx "HANDLE-PATH $ROOT/to-trunc" <<<"$output"
	grep -Fx "FILE-PATH $ROOT/to-trunc" <<<"$output"
	# The file should've been truncated by O_TRUNC.
	[ "$(stat -c '%s' "$ROOT/to-trunc")" -eq 0 ]
}

@test "root open --oflags O_RDWR|O_TRUNC [file]" {
	ROOT="$(setup_tmpdir)"

	echo "THIS SHOULD BE TRUNCATED" >"$ROOT/to-trunc"
	[ "$(stat -c '%s' "$ROOT/to-trunc")" -ne 0 ]

	pathrs-cmd root --root "$ROOT" open --oflags O_RDWR,O_TRUNC to-trunc
	[ "$status" -eq 0 ]
	! grep -Fx "HANDLE-PATH $ROOT/to-trunc" <<<"$output"
	grep -Fx "FILE-PATH $ROOT/to-trunc" <<<"$output"
	# The file should've been truncated by O_TRUNC.
	[ "$(stat -c '%s' "$ROOT/to-trunc")" -eq 0 ]
}

@test "root resolve [directory]" {
	ROOT="$(setup_tmpdir)"

	mkdir -p "$ROOT/some/random/dir"

	pathrs-cmd root --root "$ROOT" resolve some/../some/./random/dir/..
	[ "$status" -eq 0 ]
	grep -Fx "HANDLE-PATH $ROOT/some/random" <<<"$output"
	! grep '^FILE-PATH' <<<"$output"
}

@test "root resolve --reopen [directory]" {
	ROOT="$(setup_tmpdir)"

	mkdir -p "$ROOT/some/random/dir"

	pathrs-cmd root --root "$ROOT" resolve --reopen O_DIRECTORY some/random/dir
	[ "$status" -eq 0 ]
	grep -Fx "HANDLE-PATH $ROOT/some/random/dir" <<<"$output"
	grep -Fx "FILE-PATH $ROOT/some/random/dir" <<<"$output"

	mkdir -p "$ROOT/some/random/dir"
	pathrs-cmd root --root "$ROOT" resolve --reopen O_WRONLY some
	check-errno EISDIR
}

@test "root open [directory]" {
	ROOT="$(setup_tmpdir)"

	mkdir -p "$ROOT/some/random/dir"

	pathrs-cmd root --root "$ROOT" open --oflags O_DIRECTORY some/random/dir
	[ "$status" -eq 0 ]
	! grep -Fx "HANDLE-PATH $ROOT/some/random/dir" <<<"$output"
	grep -Fx "FILE-PATH $ROOT/some/random/dir" <<<"$output"

	mkdir -p "$ROOT/some/random/dir"
	pathrs-cmd root --root "$ROOT" open --oflags O_WRONLY some
	check-errno EISDIR
}

@test "root resolve [device inode]" {
	requires can-mkwhiteout

	ROOT="$(setup_tmpdir)"
	mknod "$ROOT/char-0-0" c 0 0

	pathrs-cmd root --root "$ROOT" resolve char-0-0
	[ "$status" -eq 0 ]
	grep -Fx "HANDLE-PATH $ROOT/char-0-0" <<<"$output"
	! grep '^FILE-PATH' <<<"$output"
}

@test "root resolve [/dev/full]" {
	pathrs-cmd root --root /dev resolve full
	[ "$status" -eq 0 ]
	grep -Fx 'HANDLE-PATH /dev/full' <<<"$output"
	! grep '^FILE-PATH' <<<"$output"
}

@test "root resolve [fifo]" {
	ROOT="$(setup_tmpdir)"
	mkfifo "$ROOT/fifo"

	pathrs-cmd root --root "$ROOT" resolve fifo
	[ "$status" -eq 0 ]
	grep -Fx "HANDLE-PATH $ROOT/fifo" <<<"$output"
	! grep '^FILE-PATH' <<<"$output"
}

@test "root resolve --follow [symlinks]" {
	ROOT="$(setup_tmpdir)"

	mkdir -p "$ROOT/target/dir"
	echo "TARGET" >"$ROOT/target/dir/file"
	[ "$(stat -c '%s' "$ROOT/target/dir/file")" -ne 0 ]

	ln -s /target/dir "$ROOT/a"
	ln -s /a/../dir "$ROOT/b"
	ln -s ../b/. "$ROOT/c"
	ln -s /../../../../c "$ROOT/d"
	ln -s d "$ROOT/e"
	ln -s e/file "$ROOT/file-link"

	pathrs-cmd root --root "$ROOT" resolve --follow "target/dir"
	[ "$status" -eq 0 ]
	grep -Fx "HANDLE-PATH $ROOT/target/dir" <<<"$output"
	! grep '^FILE-PATH' <<<"$output"
	pathrs-cmd root --root "$ROOT" resolve --follow "a"
	[ "$status" -eq 0 ]
	grep -Fx "HANDLE-PATH $ROOT/target/dir" <<<"$output"
	! grep '^FILE-PATH' <<<"$output"
	pathrs-cmd root --root "$ROOT" resolve --follow "b"
	[ "$status" -eq 0 ]
	grep -Fx "HANDLE-PATH $ROOT/target/dir" <<<"$output"
	! grep '^FILE-PATH' <<<"$output"
	pathrs-cmd root --root "$ROOT" resolve --follow "c"
	[ "$status" -eq 0 ]
	grep -Fx "HANDLE-PATH $ROOT/target/dir" <<<"$output"
	! grep '^FILE-PATH' <<<"$output"
	pathrs-cmd root --root "$ROOT" resolve --follow "d"
	[ "$status" -eq 0 ]
	grep -Fx "HANDLE-PATH $ROOT/target/dir" <<<"$output"
	! grep '^FILE-PATH' <<<"$output"
	pathrs-cmd root --root "$ROOT" resolve --follow "e"
	[ "$status" -eq 0 ]
	grep -Fx "HANDLE-PATH $ROOT/target/dir" <<<"$output"
	! grep '^FILE-PATH' <<<"$output"

	pathrs-cmd root --root "$ROOT" resolve --follow --reopen O_DIRECTORY "e"
	[ "$status" -eq 0 ]
	grep -Fx "HANDLE-PATH $ROOT/target/dir" <<<"$output"
	grep -Fx "FILE-PATH $ROOT/target/dir" <<<"$output"

	pathrs-cmd root --root "$ROOT" resolve --follow "file-link"
	[ "$status" -eq 0 ]
	grep -Fx "HANDLE-PATH $ROOT/target/dir/file" <<<"$output"
	! grep '^FILE-PATH' <<<"$output"

	pathrs-cmd root --root "$ROOT" resolve --follow --reopen O_WRONLY,O_TRUNC "file-link"
	[ "$status" -eq 0 ]
	grep -Fx "HANDLE-PATH $ROOT/target/dir/file" <<<"$output"
	grep -Fx "FILE-PATH $ROOT/target/dir/file" <<<"$output"
	# The file should've been truncated by O_TRUNC.
	[ "$(stat -c '%s' "$ROOT/target/dir/file")" -eq 0 ]
}

@test "root resolve --no-follow [symlinks]" {
	ROOT="$(setup_tmpdir)"

	mkdir -p "$ROOT/target/dir"
	echo "TARGET" >"$ROOT/target/dir/file"
	[ "$(stat -c '%s' "$ROOT/target/dir/file")" -ne 0 ]

	ln -s /target/dir "$ROOT/a"
	ln -s /a/../dir "$ROOT/b"
	ln -s ../b/. "$ROOT/c"
	ln -s /../../../../c "$ROOT/d"
	ln -s d "$ROOT/e"
	ln -s e/file "$ROOT/file-link"

	pathrs-cmd root --root "$ROOT" resolve --follow "target/dir"
	[ "$status" -eq 0 ]
	grep -Fx "HANDLE-PATH $ROOT/target/dir" <<<"$output"
	! grep '^FILE-PATH' <<<"$output"
	pathrs-cmd root --root "$ROOT" resolve --no-follow "a"
	[ "$status" -eq 0 ]
	grep -Fx "HANDLE-PATH $ROOT/a" <<<"$output"
	! grep '^FILE-PATH' <<<"$output"
	pathrs-cmd root --root "$ROOT" resolve --no-follow "b"
	[ "$status" -eq 0 ]
	grep -Fx "HANDLE-PATH $ROOT/b" <<<"$output"
	! grep '^FILE-PATH' <<<"$output"
	pathrs-cmd root --root "$ROOT" resolve --no-follow "c"
	[ "$status" -eq 0 ]
	grep -Fx "HANDLE-PATH $ROOT/c" <<<"$output"
	! grep '^FILE-PATH' <<<"$output"
	pathrs-cmd root --root "$ROOT" resolve --no-follow "d"
	[ "$status" -eq 0 ]
	grep -Fx "HANDLE-PATH $ROOT/d" <<<"$output"
	! grep '^FILE-PATH' <<<"$output"
	pathrs-cmd root --root "$ROOT" resolve --no-follow "e"
	[ "$status" -eq 0 ]
	grep -Fx "HANDLE-PATH $ROOT/e" <<<"$output"
	! grep '^FILE-PATH' <<<"$output"

	# You cannot reopen an O_PATH|O_NOFOLLOW handle to a symlink.
	pathrs-cmd root --root "$ROOT" resolve --no-follow --reopen O_RDONLY "e"
	check-errno ELOOP
	pathrs-cmd root --root "$ROOT" resolve --no-follow --reopen O_DIRECTORY "e"
	check-errno ELOOP

	pathrs-cmd root --root "$ROOT" resolve --no-follow "file-link"
	[ "$status" -eq 0 ]
	grep -Fx "HANDLE-PATH $ROOT/file-link" <<<"$output"
	! grep '^FILE-PATH' <<<"$output"

	pathrs-cmd root --root "$ROOT" resolve --no-follow --reopen O_WRONLY,O_TRUNC "file-link"
	check-errno ELOOP
	# The file should NOT have been truncated by O_TRUNC.
	[ "$(stat -c '%s' "$ROOT/target/dir/file")" -ne 0 ]

	# --no-follow has no impact on non-final components.
	pathrs-cmd root --root "$ROOT" resolve --no-follow --reopen O_WRONLY,O_TRUNC "e/file"
	[ "$status" -eq 0 ]
	grep -Fx "HANDLE-PATH $ROOT/target/dir/file" <<<"$output"
	grep -Fx "FILE-PATH $ROOT/target/dir/file" <<<"$output"
	# The file should've been truncated by O_TRUNC.
	[ "$(stat -c '%s' "$ROOT/target/dir/file")" -eq 0 ]
}
