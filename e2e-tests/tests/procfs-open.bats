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

# TODO: All of these tests are very limited because we cannot verify anything
# useful about the opened files (and the path is actually not guaranteed to be
# correct outside of magic-link cases). Ideally we would instead output
# something simple like the fdinfo, stat, and/or contents to verify against.

@test "procfs --base open --oflags O_RDONLY" {
	pathrs-cmd procfs open modules
	[ "$status" -eq 0 ]
	grep -E '^FILE-PATH (/proc)?/modules$' <<<"$output"

	pathrs-cmd procfs --base root open --oflags O_RDONLY modules
	[ "$status" -eq 0 ]
	grep -E '^FILE-PATH (/proc)?/modules$' <<<"$output"
}

@test "procfs --base pid=\$\$ --oflags O_RDONLY" {
	pathrs-cmd procfs open modules
	[ "$status" -eq 0 ]
	grep -E '^FILE-PATH (/proc)?/modules$' <<<"$output"

	pathrs-cmd procfs --base root open --oflags O_RDONLY modules
	[ "$status" -eq 0 ]
	grep -E '^FILE-PATH (/proc)?/modules$' <<<"$output"
}

@test "procfs --base self --oflags O_RDONLY" {
	pathrs-cmd procfs --base self open status
	[ "$status" -eq 0 ]
	grep -E '^FILE-PATH (/proc)?/[0-9]+/status$' <<<"$output"

	pathrs-cmd procfs --base self open stack
	[ "$status" -eq 0 ]
	# NOTE: While only admins can read from /proc/$n/stack, we can open it.
	grep -E '^FILE-PATH (/proc)?/[0-9]+/stack$' <<<"$output"
}

@test "procfs --base thread-self --oflags O_RDONLY" {
	pathrs-cmd procfs --base thread-self open status
	[ "$status" -eq 0 ]
	grep -E '^FILE-PATH (/proc)?/[0-9]+/task/[0-9]+/status$' <<<"$output"

	pathrs-cmd procfs --base thread-self open stack
	[ "$status" -eq 0 ]
	# NOTE: While only admins can read from /proc/$n/stack, we can open it.
	grep -E '^FILE-PATH (/proc)?/[0-9]+/task/[0-9]+/stack$' <<<"$output"
}

# Make sure that thread-self and self are actually handled differently.
@test "procfs open [self != thread-self]" {
	pathrs-cmd procfs --base self open task
	[ "$status" -eq 0 ]
	grep -E '^FILE-PATH (/proc)?/[0-9]*/task$' <<<"$output"

	pathrs-cmd procfs --base thread-self open task
	check-errno ENOENT
}

@test "procfs open --follow [symlinks]" {
	# FIXME FIXME FIXME
	[ "$(id -u)" -ne 0 ] || skip "known bug <https://github.com/cyphar/libpathrs/issues/275>"

	pathrs-cmd procfs open mounts
	[ "$status" -eq 0 ]
	grep -E '^FILE-PATH (/proc)?/[0-9]+/mounts$' <<<"$output"

	pathrs-cmd procfs open --oflags O_DIRECTORY mounts
	check-errno ENOTDIR

	pathrs-cmd procfs open --oflags O_DIRECTORY net
	[ "$status" -eq 0 ]
	grep -E '^FILE-PATH (/proc)?/[0-9]+/net$' <<<"$output"

	pathrs-cmd procfs open --follow self
	[ "$status" -eq 0 ]
	grep -E '^FILE-PATH (/proc)?/[0-9]+$' <<<"$output"

	pathrs-cmd procfs open --follow thread-self
	if [ -e /proc/thread-self ]; then
		[ "$status" -eq 0 ]
		grep -E '^FILE-PATH (/proc)?/[0-9]+/task/[0-9]+$' <<<"$output"
	else
		check-errno ENOENT
	fi
}

@test "procfs open --follow [magic-links]" {
	exepath="$(readlink -f "$PATHRS_CMD")"

	pathrs-cmd procfs --base self open --follow --oflags O_RDONLY exe
	[ "$status" -eq 0 ]
	! grep -E '^FILE-PATH (/proc)?/[0-9]+/.*$' <<<"$output"
	# We can only guess /proc/self/exe for compiled pathrs-cmd binaries.
	if is-compiled "$PATHRS_CMD"; then
		grep -Fx "FILE-PATH $exepath" <<<"$output"
	fi

	realpwd="$(readlink -f "$PWD")"

	pathrs-cmd procfs --base self open --follow --oflags O_RDONLY cwd
	[ "$status" -eq 0 ]
	grep -Fx "FILE-PATH $realpwd" <<<"$output"

	dummyfile="$(setup_tmpdir)/dummyfile"
	echo "THIS SHOULD BE TRUNCATED" >"$dummyfile"
	[ "$(stat -c '%s' "$dummyfile")" -ne 0 ]

	pathrs-cmd procfs --base thread-self open --follow --oflags O_RDWR,O_TRUNC fd/100 100>>"$dummyfile"
	[ "$status" -eq 0 ]
	grep -Fx "FILE-PATH $dummyfile" <<<"$output"
	# The file should've been truncated by O_TRUNC.
	[ "$(stat -c '%s' "$dummyfile")" -eq 0 ]
}

@test "procfs open --no-follow [symlinks]" {
	pathrs-cmd procfs open --no-follow mounts
	check-errno ELOOP

	pathrs-cmd procfs open --no-follow net
	check-errno ELOOP

	pathrs-cmd procfs open --no-follow self
	check-errno ELOOP

	pathrs-cmd procfs open --no-follow thread-self
	if [ -e /proc/thread-self ]; then
		check-errno ELOOP
	else
		check-errno ENOENT
	fi
}

@test "procfs open --no-follow [magic-links]" {
	pathrs-cmd procfs --base pid=1 open --no-follow --oflags O_DIRECTORY root
	# O_DIRECTORY beats O_NOFOLLOW!
	check-errno ENOTDIR

	pathrs-cmd procfs --base pid=1 open --no-follow --oflags O_RDWR root
	# O_NOFOLLOW beats permission errors
	check-errno ELOOP

	pathrs-cmd procfs --base self open --no-follow --oflags O_RDONLY exe
	check-errno ELOOP

	pathrs-cmd procfs --base thread-self open --no-follow --oflags O_RDONLY exe
	check-errno ELOOP

	dummyfile="$(setup_tmpdir)/dummyfile"
	echo "THIS SHOULD NOT BE TRUNCATED" >"$dummyfile"
	[ "$(stat -c '%s' "$dummyfile")" -ne 0 ]

	pathrs-cmd procfs --base thread-self open --no-follow --oflags O_RDWR,O_TRUNC fd/100 100>>"$dummyfile"
	check-errno ELOOP
	# The file should NOT have been truncated by O_TRUNC.
	[ "$(stat -c '%s' "$dummyfile")" -ne 0 ]
}

@test "procfs open --no-follow --oflags O_PATH [symlinks]" {
	pathrs-cmd procfs --base self open --no-follow --oflags O_PATH exe
	[ "$status" -eq 0 ]
	grep -E '^FILE-PATH (/proc)?/[0-9]+/exe$' <<<"$output"

	pathrs-cmd procfs --base pid=$$ open --oflags O_PATH,O_NOFOLLOW exe
	[ "$status" -eq 0 ]
	grep -E '^FILE-PATH (/proc)?/'"$$"'/exe$' <<<"$output"

	pathrs-cmd procfs --base thread-self open --oflags O_PATH,O_NOFOLLOW exe
	[ "$status" -eq 0 ]
	grep -E '^FILE-PATH (/proc)?/[0-9]+/task/[0-9]+/exe$' <<<"$output"
}

@test "procfs open [symlink parent component]" {
	pathrs-cmd procfs --base root open self/status
	[ "$status" -eq 0 ]
	grep -E '^FILE-PATH (/proc)?/[0-9]+/status$' <<<"$output"

	pathrs-cmd procfs --base root open self/fdinfo/0
	[ "$status" -eq 0 ]
	grep -E '^FILE-PATH (/proc)?/[0-9]+/fdinfo/0$' <<<"$output"

	# FIXME FIXME FIXME
	skip "known bug <https://github.com/cyphar/libpathrs/issues/274>"

	exepath="$(readlink -f "$PATHRS_CMD")"

	pathrs-cmd procfs --base root open self/exe
	[ "$status" -eq 0 ]
	! grep -E '^FILE-PATH (/proc)?/[0-9]+/.*$' <<<"$output"
	# We can only guess /proc/self/exe for compiled pathrs-cmd binaries.
	if is-compiled "$PATHRS_CMD"; then
		grep -Fx "FILE-PATH $exepath" <<<"$output"
	fi
}
