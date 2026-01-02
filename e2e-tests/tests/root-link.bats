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

@test "root symlink" {
	ROOT="$(setup_tmpdir)"

	mkdir -p "$ROOT/etc"
	echo "passwd" >"$ROOT/etc/passwd"

	pathrs-cmd root --root "$ROOT" symlink /etc/passwd passwd-link
	[ "$status" -eq 0 ]

	[ -f "$ROOT/etc/passwd" ]
	[ -L "$ROOT/passwd-link" ]

	sane_run readlink "$ROOT/passwd-link"
	[ "$status" -eq 0 ]
	[[ "$output" == "/etc/passwd" ]]

	pathrs-cmd root --root "$ROOT" readlink /passwd-link
	[ "$status" -eq 0 ]
	grep -Fx 'LINK-TARGET /etc/passwd' <<<"$output"
}

@test "root symlink [no-clobber file]" {
	ROOT="$(setup_tmpdir)"

	mkdir -p "$ROOT/etc"
	touch "$ROOT/link"

	ino="$(stat -c '%i' "$ROOT/link")"

	pathrs-cmd root --root "$ROOT" symlink ../target link
	check-errno EEXIST

	[ -f "$ROOT/link" ]
	sane_run stat -c '%i' "$ROOT/link"
	[[ "$output" == "$ino" ]]
}

@test "root symlink [no-clobber symlink]" {
	ROOT="$(setup_tmpdir)"

	mkdir -p "$ROOT/etc"
	touch "$ROOT/new"
	ln -s /old/link "$ROOT/link"

	pathrs-cmd root --root "$ROOT" symlink /new link
	check-errno EEXIST

	[ -L "$ROOT/link" ]

	sane_run readlink "$ROOT/link"
	[ "$status" -eq 0 ]
	[[ "$output" == "/old/link" ]]

	pathrs-cmd root --root "$ROOT" readlink /link
	[ "$status" -eq 0 ]
	grep -Fx 'LINK-TARGET /old/link' <<<"$output"
}

@test "root symlink [directory]" {
	ROOT="$(setup_tmpdir)"

	mkdir -p "$ROOT/foo/bar/baz"

	pathrs-cmd root --root "$ROOT" symlink ../foo/bar link
	[ "$status" -eq 0 ]

	[ -d "$ROOT/foo/bar" ]
	[ -L "$ROOT/link" ]

	sane_run readlink "$ROOT/link"
	[ "$status" -eq 0 ]
	[[ "$output" == "../foo/bar" ]]

	pathrs-cmd root --root "$ROOT" readlink link
	[ "$status" -eq 0 ]
	grep -Fx 'LINK-TARGET ../foo/bar' <<<"$output"
}

@test "root symlink [symlink]" {
	ROOT="$(setup_tmpdir)"

	mkdir -p "$ROOT/foo"
	touch "$ROOT/foo/bar"
	ln -s /foo/bar "$ROOT/target-link"

	pathrs-cmd root --root "$ROOT" symlink target-link link
	[ "$status" -eq 0 ]

	[ -f "$ROOT/foo/bar" ]
	[ -L "$ROOT/target-link" ]
	[ -L "$ROOT/link" ]

	sane_run readlink "$ROOT/link"
	[ "$status" -eq 0 ]
	[[ "$output" == "target-link" ]]

	pathrs-cmd root --root "$ROOT" readlink link
	[ "$status" -eq 0 ]
	grep -Fx 'LINK-TARGET target-link' <<<"$output"
}

@test "root symlink [non-existent target]" {
	ROOT="$(setup_tmpdir)"

	pathrs-cmd root --root "$ROOT" symlink ../..//some/dummy/path link
	[ "$status" -eq 0 ]

	! [ -e "$ROOT/some/dummy/path" ]
	[ -L "$ROOT/link" ]

	sane_run readlink "$ROOT/link"
	[ "$status" -eq 0 ]
	[[ "$output" == "../..//some/dummy/path" ]]

	pathrs-cmd root --root "$ROOT" readlink link
	[ "$status" -eq 0 ]
	grep -Fx 'LINK-TARGET ../..//some/dummy/path' <<<"$output"
}

@test "root symlink [non-existent parent component]" {
	ROOT="$(setup_tmpdir)"

	mkdir -p "$ROOT/foo/bar/baz"
	mkdir -p "$ROOT/etc"
	echo "passwd" >"$ROOT/etc/passwd"

	pathrs-cmd root --root "$ROOT" symlink /etc/passwd foo/nope/baz
	check-errno ENOENT

	! [ -e "$ROOT/foo/nope" ]
	! [ -e "$ROOT/foo/nope/baz" ]
}

@test "root symlink [bad parent component]" {
	ROOT="$(setup_tmpdir)"

	mkdir -p "$ROOT/foo"
	touch "$ROOT/foo/bar"
	mkdir -p "$ROOT/etc"
	echo "passwd" >"$ROOT/etc/passwd"

	pathrs-cmd root --root "$ROOT" symlink /etc/passwd foo/bar/baz
	check-errno ENOTDIR

	[ -f "$ROOT/foo/bar" ]
	! [ -e "$ROOT/foo/bar/baz" ]
}

@test "root hardlink" {
	ROOT="$(setup_tmpdir)"

	mkdir -p "$ROOT/etc"
	echo "passwd" >"$ROOT/etc/passwd"

	pathrs-cmd root --root "$ROOT" hardlink /etc/passwd passwd-link
	[ "$status" -eq 0 ]

	[ -f "$ROOT/etc/passwd" ]
	[ -f "$ROOT/passwd-link" ]

	sane_run readlink "$ROOT/passwd-link"
	[ "$status" -ne 0 ] # not a symlink!

	# Hardlinks have the same inode.
	sane_run stat -c '%i' "$ROOT/etc/passwd"
	target_ino="$output"
	sane_run stat -c '%i' "$ROOT/passwd-link"
	link_ino="$output"
	[[ "$target_ino" == "$link_ino" ]]
}

@test "root hardlink [no-clobber file]" {
	ROOT="$(setup_tmpdir)"

	mkdir -p "$ROOT/etc"
	touch "$ROOT/target"
	touch "$ROOT/link"

	ino="$(stat -c '%i' "$ROOT/link")"

	pathrs-cmd root --root "$ROOT" hardlink target link
	check-errno EEXIST

	[ -f "$ROOT/link" ]
	sane_run stat -c '%i' "$ROOT/link"
	[[ "$output" == "$ino" ]]
}

@test "root hardlink [no-clobber symlink]" {
	ROOT="$(setup_tmpdir)"

	touch "$ROOT/foobar"
	touch "$ROOT/target"
	ln -s /foobar "$ROOT/link"

	ino="$(stat -c '%i' "$ROOT/link")"

	pathrs-cmd root --root "$ROOT" hardlink /target link
	check-errno EEXIST

	[ -L "$ROOT/link" ]
	sane_run stat -c '%i' "$ROOT/link"
	[[ "$output" == "$ino" ]]
}

@test "root hardlink [directory]" {
	ROOT="$(setup_tmpdir)"

	mkdir -p "$ROOT/foo/bar/baz"

	pathrs-cmd root --root "$ROOT" hardlink /foo/bar link
	check-errno EPERM

	[ -d "$ROOT/foo/bar" ]
	! [ -e "$ROOT/link" ]
}

@test "root hardlink [non-existent target]" {
	ROOT="$(setup_tmpdir)"

	pathrs-cmd root --root "$ROOT" hardlink ../..//some/dummy/path link
	check-errno ENOENT
}

@test "root hardlink [non-existent parent component]" {
	ROOT="$(setup_tmpdir)"

	mkdir -p "$ROOT/foo/bar/baz"
	mkdir -p "$ROOT/etc"
	echo "passwd" >"$ROOT/etc/passwd"

	pathrs-cmd root --root "$ROOT" hardlink /etc/passwd foo/nope/baz
	check-errno ENOENT

	! [ -e "$ROOT/foo/nope" ]
	! [ -e "$ROOT/foo/nope/baz" ]
}

@test "root hardlink [bad parent component]" {
	ROOT="$(setup_tmpdir)"

	mkdir -p "$ROOT/foo"
	touch "$ROOT/foo/bar"
	mkdir -p "$ROOT/etc"
	echo "passwd" >"$ROOT/etc/passwd"

	pathrs-cmd root --root "$ROOT" hardlink /etc/passwd foo/bar/baz
	check-errno ENOTDIR

	[ -f "$ROOT/foo/bar" ]
	! [ -e "$ROOT/foo/bar/baz" ]
}
