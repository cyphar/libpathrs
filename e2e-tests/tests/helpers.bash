#!/bin/bash
# SPDX-License-Identifier: MPL-2.0
#
# libpathrs: safe path resolution on Linux
# Copyright (C) 2019-2025 Aleksa Sarai <cyphar@cyphar.com>
# Copyright (C) 2019-2025 SUSE LLC
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

set -u

function fail() {
	echo "FAILURE:" "$@" >&2
	false
}

[ -v PATHRS_CMD ] || {
	echo "PATHRS_CMD (pathrs-cmd) must be provided" >&2
	exit 2
}

# Like bats's built-in run, except that we get output and status information.
function sane_run() {
	local cmd="$1"
	shift

	run "$cmd" "$@"

	# Some debug information to make life easier.
	echo "$(basename "$cmd") $* (status=$status)" >&2
	echo "$output" >&2
}

# Wrapper for PATHRS_CMD.
function pathrs-cmd() {
	sane_run "$PATHRS_CMD" "$@"
}

# Shorthand for checking the ERRNO value from pathrs-cmd.
function check-errno() {
	local errno_num
	errno_num="$(errno "$1" | awk '{ print $2 }')"
	[ "$status" -ne 0 ]
	[[ "$output" == *"ERRNO $errno_num "* ]]
}

# Let's not store everything in /tmp -- that would just be messy.
TESTDIR_TMPDIR="$BATS_TMPDIR/pathrs-e2e-tests"
mkdir -p "$TESTDIR_TMPDIR"

# Stores the set of tmpdirs that still have to be cleaned up. Calling
# teardown_tmpdirs will set this to an empty array (and all the tmpdirs
# contained within are removed).
TESTDIR_LIST="$TESTDIR_TMPDIR/pathrs-test-tmpdirs.$$"

# setup_tmpdir creates a new temporary directory and returns its name.  Note
# that if "$IS_ROOTLESS" is true, then removing this tmpdir might be harder
# than expected -- so tests should not really attempt to clean up tmpdirs.
function setup_tmpdir() {
	[[ -n "${PATHRS_TMPDIR:-}" ]] || PATHRS_TMPDIR="$TESTDIR_TMPDIR"
	mktemp -d "$PATHRS_TMPDIR/pathrs-test-tmpdir.XXXXXXXX" | tee -a "$TESTDIR_LIST"
}

# setup_tmpdirs just sets up the "built-in" tmpdirs.
function setup_tmpdirs() {
	declare -g PATHRS_TMPDIR
	PATHRS_TMPDIR="$(setup_tmpdir)"
}

# teardown_tmpdirs removes all tmpdirs created with setup_tmpdir.
function teardown_tmpdirs() {
	# Do nothing if TESTDIR_LIST doesn't exist.
	[ -e "$TESTDIR_LIST" ] || return

	# Remove all of the tmpdirs.
	while IFS= read -r tmpdir; do
		[ -e "$tmpdir" ] || continue
		chmod -R 0777 "$tmpdir"
		rm -rf "$tmpdir"
	done < "$TESTDIR_LIST"

	# Clear tmpdir list.
	rm -f "$TESTDIR_LIST"
}

# Returns whether the provided binary is compiled or is a #!-script. This is
# necessary for tests that look at /proc/self/exe, because for those we cannot
# just take the expected path as being $PATHRS_CMD.
function is-compiled() {
	local bin="$1"
	readelf -h "$bin" >/dev/null 2>&1
}

# Allows a test to specify what things it requires. If the environment can't
# support it, the test is skipped with a message.
function requires() {
	for var in "$@"; do
		case "$var" in
			root)
				[ "$(id -u)" -eq 0 ] || skip "test requires ${var}"
				;;
			compiled)
				{ is-compiled "$1"; } || skip "test requires $PATHRS_CMD be compiled"
				;;
			can-mkwhiteout)
				mknod "$(setup_tmpdir)/mknod0" c 0 0 || skip "test requires the ability to 'mknod c 0 0'"
				;;
			*)
				fail "BUG: Invalid requires ${var}."
				;;
		esac
	done
}
