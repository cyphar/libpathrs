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
	# EINVAL => Non-symlink.
	pathrs-cmd procfs --base self readlink task
	check-errno EINVAL
	[[ "$output" == *"error:"*"readlinkat"* ]] # Make sure the error is from readlinkat(2).

	# ENOENT => No such path.
	pathrs-cmd procfs --base thread-self readlink task
	check-errno ENOENT
	[[ "$output" != *"error:"*"readlinkat"* ]] # Make sure the error is NOT from readlinkat(2).
}

@test "procfs readlink [enoent]" {
	pathrs-cmd procfs readlink non-exist
	check-errno ENOENT
	[[ "$output" != *"error:"*"readlinkat"* ]] # Make sure the error is NOT from readlinkat(2).
}

@test "procfs readlink [file einval]" {
	pathrs-cmd procfs readlink uptime
	check-errno EINVAL
	[[ "$output" == *"error:"*"readlinkat"* ]] # Make sure the error is from readlinkat(2).

	pathrs-cmd procfs --base root readlink tty/drivers
	check-errno EINVAL
	[[ "$output" == *"error:"*"readlinkat"* ]] # Make sure the error is from readlinkat(2).

	pathrs-cmd procfs --base root readlink self/fdinfo/0
	check-errno EINVAL
	[[ "$output" == *"error:"*"readlinkat"* ]] # Make sure the error is from readlinkat(2).

	pathrs-cmd procfs --base pid=1 readlink stat
	check-errno EINVAL
	[[ "$output" == *"error:"*"readlinkat"* ]] # Make sure the error is from readlinkat(2).

	pathrs-cmd procfs --base self readlink status
	check-errno EINVAL
	[[ "$output" == *"error:"*"readlinkat"* ]] # Make sure the error is from readlinkat(2).

	pathrs-cmd procfs --base thread-self readlink stack
	check-errno EINVAL
	[[ "$output" == *"error:"*"readlinkat"* ]] # Make sure the error is from readlinkat(2).
}

@test "procfs readlink [dir einval]" {
	pathrs-cmd procfs readlink tty
	check-errno EINVAL
	[[ "$output" == *"error:"*"readlinkat"* ]] # Make sure the error is from readlinkat(2).

	pathrs-cmd procfs --base root readlink self/fdinfo
	check-errno EINVAL
	[[ "$output" == *"error:"*"readlinkat"* ]] # Make sure the error is from readlinkat(2).
}
