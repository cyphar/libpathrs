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

function setup() {
	setup_tmpdirs
}

function teardown() {
	teardown_tmpdirs
}

@test "root unlink [non-existent]" {
	ROOT="$(setup_tmpdir)"

	pathrs-cmd root --root "$ROOT" unlink non-exist
	check-errno ENOENT
}

@test "root rmdir [non-existent]" {
	ROOT="$(setup_tmpdir)"

	pathrs-cmd root --root "$ROOT" rmdir non-exist
	check-errno ENOENT
}

@test "root rmdir-all [non-existent]" {
	ROOT="$(setup_tmpdir)"

	pathrs-cmd root --root "$ROOT" rmdir-all non-exist
	[ "$status" -eq 0 ]
}

@test "root unlink [file]" {
	ROOT="$(setup_tmpdir)"

	echo "file" >"$ROOT/file"
	[ -e "$ROOT/file" ]

	pathrs-cmd root --root "$ROOT" unlink /file
	[ "$status" -eq 0 ]
	! [ -e "$ROOT/file" ]
}

@test "root rmdir [file]" {
	ROOT="$(setup_tmpdir)"

	echo "file" >"$ROOT/file"
	[ -e "$ROOT/file" ]

	pathrs-cmd root --root "$ROOT" rmdir /file
	check-errno ENOTDIR
	[ -e "$ROOT/file" ]
}

@test "root rmdir-all [file]" {
	ROOT="$(setup_tmpdir)"

	echo "file" >"$ROOT/file"
	[ -e "$ROOT/file" ]

	pathrs-cmd root --root "$ROOT" rmdir-all /file
	[ "$status" -eq 0 ]
	! [ -e "$ROOT/file" ]
}

@test "root unlink [fifo]" {
	ROOT="$(setup_tmpdir)"

	mkfifo "$ROOT/fifo"
	[ -e "$ROOT/fifo" ]

	pathrs-cmd root --root "$ROOT" unlink /fifo
	[ "$status" -eq 0 ]
	! [ -e "$ROOT/fifo" ]
}

@test "root rmdir [fifo]" {
	ROOT="$(setup_tmpdir)"

	mkfifo "$ROOT/fifo"
	[ -e "$ROOT/fifo" ]

	pathrs-cmd root --root "$ROOT" rmdir /fifo
	check-errno ENOTDIR
	[ -e "$ROOT/fifo" ]
}

@test "root rmdir-all [fifo]" {
	ROOT="$(setup_tmpdir)"

	mkfifo "$ROOT/fifo"
	[ -e "$ROOT/fifo" ]

	pathrs-cmd root --root "$ROOT" rmdir-all /fifo
	[ "$status" -eq 0 ]
	! [ -e "$ROOT/fifo" ]
}

@test "root unlink [symlink]" {
	ROOT="$(setup_tmpdir)"

	touch "$ROOT/file"
	ln -s file "$ROOT/alt"

	pathrs-cmd root --root "$ROOT" unlink alt
	[ "$status" -eq 0 ]
	! [ -e "$ROOT/alt" ]
	[ -e "$ROOT/file" ]
}

@test "root rmdir [symlink]" {
	ROOT="$(setup_tmpdir)"

	mkdir -p "$ROOT/some/directory"
	ln -s some "$ROOT/alt"

	pathrs-cmd root --root "$ROOT" rmdir-all alt
	[ "$status" -eq 0 ]
	! [ -e "$ROOT/alt" ]
	[ -e "$ROOT/some" ]
	[ -e "$ROOT/some/directory" ]
}

@test "root rmdir-all [symlink]" {
	ROOT="$(setup_tmpdir)"

	mkdir -p "$ROOT/some/directory"
	ln -s some "$ROOT/alt"

	pathrs-cmd root --root "$ROOT" rmdir-all alt
	[ "$status" -eq 0 ]
	! [ -e "$ROOT/alt" ]
	[ -e "$ROOT/some" ]
	[ -e "$ROOT/some/directory" ]
}

@test "root unlink [directory]" {
	ROOT="$(setup_tmpdir)"

	mkdir -p "$ROOT/some/directory"

	pathrs-cmd root --root "$ROOT" unlink ../some/directory
	check-errno EISDIR
	[ -e "$ROOT/some/directory" ]

	pathrs-cmd root --root "$ROOT" unlink ../some
	check-errno EISDIR
	[ -e "$ROOT/some/directory" ]
}

@test "root rmdir [directory]" {
	ROOT="$(setup_tmpdir)"

	mkdir -p "$ROOT/some/directory"

	pathrs-cmd root --root "$ROOT" rmdir ../some
	check-errno ENOTEMPTY
	[ -e "$ROOT/some" ]

	pathrs-cmd root --root "$ROOT" rmdir ../some/directory
	[ "$status" -eq 0 ]
	! [ -e "$ROOT/some/directory" ]

	pathrs-cmd root --root "$ROOT" rmdir ../some
	[ "$status" -eq 0 ]
	! [ -e "$ROOT/some" ]
}

@test "root rmdir-all [directory]" {
	ROOT="$(setup_tmpdir)"

	mkdir -p "$ROOT/some/directory"
	touch "$ROOT/some/file"

	pathrs-cmd root --root "$ROOT" rmdir-all ../some
	[ "$status" -eq 0 ]
	! [ -e "$ROOT/some" ]
	! [ -e "$ROOT/some/file" ]
	! [ -e "$ROOT/some/directory" ]
}

@test "root rmdir-all [big tree]" {
	ROOT="$(setup_tmpdir)"

	# Make a fairly deep tree.
	for dir1 in $(seq 16); do
		for dir2 in $(seq 32); do
			subdir="tree/dir-$dir1/dirdir-$dir2"
			mkdir -p "$ROOT/$subdir"
			for file in $(seq 16); do
				echo "file $file in $subdir" >"$ROOT/$subdir/file-$file-$RANDOM"
			done
		done
	done

	pathrs-cmd root --root "$ROOT" rmdir-all tree
	[ "$status" -eq 0 ]
	! [ -e "$ROOT/tree" ]
}

@test "root rmdir-all ." {
	# FIXME FIXME FIXME
	skip "known bug <https://github.com/cyphar/libpathrs/issues/277>"

	ROOT="$(setup_tmpdir)"

	# Make a fairly deep tree.
	for dir1 in $(seq 16); do
		for dir2 in $(seq 16); do
			subdir="dir-$dir1/dirdir-$dir2"
			mkdir -p "$ROOT/$subdir"
			for file in $(seq 16); do
				echo "file $file in $subdir" >"$ROOT/$subdir/file-$file-$RANDOM"
			done
		done
	done

	pathrs-cmd root --root "$ROOT" rmdir-all .
	[ "$status" -eq 0 ]
	! [ -e "$ROOT/tree" ]
	# The top-level root should not be removed.
	[ -d "$ROOT" ]
}

@test "root rmdir-all /" {
	# FIXME FIXME FIXME
	skip "known bug <https://github.com/cyphar/libpathrs/issues/277>"

	ROOT="$(setup_tmpdir)"

	# Make a fairly deep tree.
	for dir1 in $(seq 16); do
		for dir2 in $(seq 16); do
			subdir="dir-$dir1/dirdir-$dir2"
			mkdir -p "$ROOT/$subdir"
			for file in $(seq 16); do
				echo "file $file in $subdir" >"$ROOT/$subdir/file-$file-$RANDOM"
			done
		done
	done

	pathrs-cmd root --root "$ROOT" rmdir-all /
	[ "$status" -eq 0 ]
	! [ -e "$ROOT/tree" ]
	# The top-level root should not be removed.
	[ -d "$ROOT" ]
}

# TODO: Add a test for removing rootless-unfriendly chmod 000 directories.
# See <https://github.com/cyphar/libpathrs/issues/279>.
