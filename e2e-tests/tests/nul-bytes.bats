#!/usr/bin/bats -t
# SPDX-License-Identifier: MPL-2.0
#
# libpathrs: safe path resolution on Linux
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

# Paths with embedded nul bytes must be rejected by the bindings rather than
# being silently truncated when converted to C strings. The paths used in these
# tests are chosen such that the truncated path would succeed, so a binding
# that truncates paths would fail these tests.

@test "pathrs-cmd unescape [sanity check]" {
	ROOT="$(setup_tmpdir)"
	touch "$ROOT/back\\slash"

	pathrs-cmd-unescape root --root "$ROOT" resolve 'back\\slash'
	[ "$status" -eq 0 ]
	grep -Fx "HANDLE-PATH $ROOT/back\\slash" <<<"$output"

	pathrs-cmd-unescape root --root "$ROOT" resolve 'back\slash'
	[ "$status" -ne 0 ]
}

@test "root open [nul byte in root path]" {
	ROOT="$(setup_tmpdir)"
	mkdir "$ROOT/a"

	pathrs-cmd-unescape root --root "$ROOT"'\0/garbage' resolve a
	check-errno EINVAL
}

@test "root resolve [nul byte]" {
	ROOT="$(setup_tmpdir)"
	echo "dummy" >"$ROOT/file"
	ln -s file "$ROOT/link"

	pathrs-cmd-unescape root --root "$ROOT" resolve 'file\0garbage'
	check-errno EINVAL

	pathrs-cmd-unescape root --root "$ROOT" resolve --no-follow 'link\0garbage'
	check-errno EINVAL

	pathrs-cmd-unescape root --root "$ROOT" resolve '\0garbage'
	check-errno EINVAL
}

@test "root open [nul byte]" {
	ROOT="$(setup_tmpdir)"
	echo "dummy" >"$ROOT/file"

	pathrs-cmd-unescape root --root "$ROOT" open --oflags O_RDONLY 'file\0garbage'
	check-errno EINVAL
}

@test "root mkfile [nul byte]" {
	ROOT="$(setup_tmpdir)"

	pathrs-cmd-unescape root --root "$ROOT" mkfile --mode 0644 'file\0garbage'
	check-errno EINVAL
	[ ! -e "$ROOT/file" ]
}

@test "root mkdir [nul byte]" {
	ROOT="$(setup_tmpdir)"

	pathrs-cmd-unescape root --root "$ROOT" mkdir --mode 0755 'dir\0garbage'
	check-errno EINVAL
	[ ! -e "$ROOT/dir" ]
}

@test "root mkdir-all [nul byte]" {
	ROOT="$(setup_tmpdir)"
	mkdir -p "$ROOT/a/b"

	pathrs-cmd-unescape root --root "$ROOT" mkdir-all --mode 0755 'foo/bar\0garbage'
	check-errno EINVAL
	[ ! -e "$ROOT/foo" ]

	pathrs-cmd-unescape root --root "$ROOT" mkdir-all --mode 0755 'a/b\0garbage/c'
	check-errno EINVAL
	[ ! -e "$ROOT/a/b/c" ]
}

@test "root mknod [nul byte]" {
	ROOT="$(setup_tmpdir)"

	pathrs-cmd-unescape root --root "$ROOT" mknod --mode 0644 'fifo\0garbage' p
	check-errno EINVAL
	[ ! -e "$ROOT/fifo" ]
}

@test "root hardlink [nul byte]" {
	ROOT="$(setup_tmpdir)"
	echo "dummy" >"$ROOT/file"

	pathrs-cmd-unescape root --root "$ROOT" hardlink 'file\0garbage' link
	check-errno EINVAL
	[ ! -e "$ROOT/link" ]

	pathrs-cmd-unescape root --root "$ROOT" hardlink file 'link\0garbage'
	check-errno EINVAL
	[ ! -e "$ROOT/link" ]
}

@test "root symlink [nul byte]" {
	ROOT="$(setup_tmpdir)"

	pathrs-cmd-unescape root --root "$ROOT" symlink '/etc\0garbage' link
	check-errno EINVAL
	[ ! -L "$ROOT/link" ]

	pathrs-cmd-unescape root --root "$ROOT" symlink /etc 'link\0garbage'
	check-errno EINVAL
	[ ! -L "$ROOT/link" ]
}

@test "root readlink [nul byte]" {
	ROOT="$(setup_tmpdir)"
	ln -s /etc "$ROOT/link"

	pathrs-cmd-unescape root --root "$ROOT" readlink 'link\0garbage'
	check-errno EINVAL
}

@test "root unlink [nul byte]" {
	ROOT="$(setup_tmpdir)"
	echo "dummy" >"$ROOT/file"

	pathrs-cmd-unescape root --root "$ROOT" unlink 'file\0garbage'
	check-errno EINVAL
	[ -e "$ROOT/file" ]
}

@test "root rmdir [nul byte]" {
	ROOT="$(setup_tmpdir)"
	mkdir "$ROOT/dir"

	pathrs-cmd-unescape root --root "$ROOT" rmdir 'dir\0garbage'
	check-errno EINVAL
	[ -d "$ROOT/dir" ]
}

@test "root rmdir-all [nul byte]" {
	ROOT="$(setup_tmpdir)"
	mkdir -p "$ROOT/dir/a/b/c"

	pathrs-cmd-unescape root --root "$ROOT" rmdir-all 'dir\0garbage'
	check-errno EINVAL
	[ -d "$ROOT/dir/a/b/c" ]
}

@test "root rename [nul byte]" {
	ROOT="$(setup_tmpdir)"
	echo "dummy" >"$ROOT/file"

	pathrs-cmd-unescape root --root "$ROOT" rename 'file\0garbage' newfile
	check-errno EINVAL
	[ -e "$ROOT/file" ]
	[ ! -e "$ROOT/newfile" ]

	pathrs-cmd-unescape root --root "$ROOT" rename file 'newfile\0garbage'
	check-errno EINVAL
	[ -e "$ROOT/file" ]
	[ ! -e "$ROOT/newfile" ]
}

@test "procfs open [nul byte]" {
	pathrs-cmd-unescape procfs open 'filesystems\0garbage'
	check-errno EINVAL

	pathrs-cmd-unescape procfs --base self open 'status\0garbage'
	check-errno EINVAL

	pathrs-cmd-unescape procfs --base self open --no-follow 'cwd\0garbage'
	check-errno EINVAL
}

@test "procfs readlink [nul byte]" {
	pathrs-cmd-unescape procfs --base self readlink 'cwd\0garbage'
	check-errno EINVAL

	pathrs-cmd-unescape procfs readlink 'self\0garbage'
	check-errno EINVAL
}
