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

@test "procfs --base root readlink" {
	pathrs-cmd procfs readlink net
	[ "$status" -eq 0 ]
	grep -Fx 'LINK-TARGET self/net' <<<"$output"

	pathrs-cmd procfs --base root readlink mounts
	[ "$status" -eq 0 ]
	grep -Fx 'LINK-TARGET self/mounts' <<<"$output"

	pathrs-cmd procfs --base root readlink self
	[ "$status" -eq 0 ]
	grep -Ex 'LINK-TARGET [0-9]+' <<<"$output"

	pathrs-cmd procfs --base root readlink thread-self
	if [ -e /proc/thread-self ]; then
		[ "$status" -eq 0 ]
		grep -Ex 'LINK-TARGET [0-9]+/task/[0-9]+' <<<"$output"
	else
		check-errno ENOENT
	fi
}

@test "procfs --base root readlink [symlink parent component]" {
	realpwd="$(readlink -f "$PWD")"

	pathrs-cmd procfs --base root readlink self/cwd
	[ "$status" -eq 0 ]
	grep -Fx "LINK-TARGET $realpwd" <<<"$output"
}

@test "procfs --base pid=\$\$ readlink" {
	realpwd="$(readlink -f "$PWD")"

	pathrs-cmd procfs --base pid=$$ readlink cwd
	[ "$status" -eq 0 ]
	grep -Fx "LINK-TARGET $realpwd" <<<"$output"
}

@test "procfs --base self readlink" {
	exepath="$(readlink -f "$PATHRS_CMD")"

	pathrs-cmd procfs --base self readlink exe
	[ "$status" -eq 0 ]
	! grep -E '^FILE-PATH (/proc)?/[0-9]+/.*$' <<<"$output"
	# We can only guess /proc/self/exe for compiled pathrs-cmd binaries.
	if is-compiled "$PATHRS_CMD"; then
		grep -Fx "LINK-TARGET $exepath" <<<"$output"
	fi

	dummyfile="$(setup_tmpdir)/dummyfile"
	touch "$dummyfile"

	pathrs-cmd procfs --base self readlink fd/100 100>"$dummyfile"
	[ "$status" -eq 0 ]
	grep -Fx "LINK-TARGET $dummyfile" <<<"$output"
}

@test "procfs --base thread-self readlink" {
	exepath="$(readlink -f "$PATHRS_CMD")"

	pathrs-cmd procfs --base thread-self readlink exe
	[ "$status" -eq 0 ]
	! grep -E '^FILE-PATH (/proc)?/[0-9]+/.*$' <<<"$output"
	# We can only guess /proc/self/exe for compiled pathrs-cmd binaries.
	if is-compiled "$PATHRS_CMD"; then
		grep -Fx "LINK-TARGET $exepath" <<<"$output"
	fi

	dummyfile="$(setup_tmpdir)/dummyfile"
	touch "$dummyfile"

	pathrs-cmd procfs --base thread-self readlink fd/100 100>"$dummyfile"
	[ "$status" -eq 0 ]
	grep -Fx "LINK-TARGET $dummyfile" <<<"$output"
}

# Make sure that thread-self and self are actually handled differently.
@test "procfs readlink [self != thread-self]" {
	# This is a little ugly, but if we get ENOENT from trying to *resolve*
	# $base/task then we know we are in thread-self. Unfortunately, readlinkat
	# also returns ENOENT if the target is not a symlink so we need to check
	# whether the error is coming from readlinkat as well.

	pathrs-cmd procfs --base self readlink task
	check-errno ENOENT
	[[ "$output" == *"error:"*"readlinkat"* ]] # Make sure the error is from readlinkat(2).

	pathrs-cmd procfs --base thread-self readlink task
	check-errno ENOENT
	[[ "$output" != *"error:"*"readlinkat"* ]] # Make sure the error is NOT from readlinkat(2).
}

@test "procfs readlink [non-symlink]" {
	pathrs-cmd procfs readlink uptime
	check-errno ENOENT
	[[ "$output" == *"error:"*"readlinkat"* ]] # Make sure the error is from readlinkat(2).

	pathrs-cmd procfs --base root readlink sys/fs/overflowuid
	check-errno ENOENT
	[[ "$output" == *"error:"*"readlinkat"* ]] # Make sure the error is from readlinkat(2).

	pathrs-cmd procfs --base pid=1 readlink stat
	check-errno ENOENT
	[[ "$output" == *"error:"*"readlinkat"* ]] # Make sure the error is from readlinkat(2).

	pathrs-cmd procfs --base self readlink status
	check-errno ENOENT
	[[ "$output" == *"error:"*"readlinkat"* ]] # Make sure the error is from readlinkat(2).

	pathrs-cmd procfs --base thread-self readlink stack
	check-errno ENOENT
	[[ "$output" == *"error:"*"readlinkat"* ]] # Make sure the error is from readlinkat(2).
}
