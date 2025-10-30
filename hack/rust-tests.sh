#!/bin/bash
# SPDX-License-Identifier: MPL-2.0 OR LGPL-3.0-or-later
#
# libpathrs: safe path resolution on Linux
# Copyright (C) 2019-2025 Aleksa Sarai <cyphar@cyphar.com>
# Copyright (C) 2019-2025 SUSE LLC
#
# == MPL-2.0 ==
#
#  This Source Code Form is subject to the terms of the Mozilla Public
#  License, v. 2.0. If a copy of the MPL was not distributed with this
#  file, You can obtain one at https://mozilla.org/MPL/2.0/.
#
# Alternatively, this Source Code Form may also (at your option) be used
# under the terms of the GNU Lesser General Public License Version 3, as
# described below:
#
# == LGPL-3.0-or-later ==
#
#  This program is free software: you can redistribute it and/or modify it
#  under the terms of the GNU Lesser General Public License as published by
#  the Free Software Foundation, either version 3 of the License, or (at
#  your option) any later version.
#
#  This program is distributed in the hope that it will be useful, but
#  WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY
#  or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License
#  for more details.
#
#  You should have received a copy of the GNU Lesser General Public License
#  along with this program. If not, see <https://www.gnu.org/licenses/>.

set -Eeuo pipefail

SRC_ROOT="$(readlink -f "$(dirname "${BASH_SOURCE[0]}")/..")"

function bail() {
	echo "rust tests: $*" >&2
	exit 1
}

function contains() {
	local elem needle="$1"
	shift
	for elem in "$@"; do
		[[ "$elem" == "$needle" ]] && return 0
	done
	return 1
}

function strjoin() {
	local sep="$1"
	shift

	local str=
	until [[ "$#" == 0 ]]; do
		str+="${1:-}"
		shift
		[[ "$#" == 0 ]] && break
		str+="$sep"
	done
	echo "$str"
}

TEMP="$(getopt -o sc:p:S: --long sudo,cargo:,partition:,enosys:,archive-file: -- "$@")"
eval set -- "$TEMP"

sudo=
partition=
enosys_syscalls=()
nextest_archive=
CARGO="${CARGO_NIGHTLY:-cargo +nightly}"
while [ "$#" -gt 0 ]; do
	case "$1" in
		--archive-file)
			nextest_archive="$2"
			shift 2
			;;
		-s|--sudo)
			sudo=1
			shift
			;;
		-c|--cargo)
			CARGO="$2"
			shift 2
			;;
		-p|--partition)
			partition="$2"
			shift 2
			;;
		-S|--enosys)
			[ -n "$2" ] && enosys_syscalls+=("$2")
			shift 2
			;;
		--)
			shift
			break
			;;
		*)
			bail "unknown option $1"
	esac
done
tests_to_run=("$@")

[ -n "$partition" ] || {
	partition=1
	# When running in GHA, if we run the *entire* test suite without splitting
	# it to compact the profraw coverage data, we run out of disk space, so by
	# default we should at least partition the run into 2. If the caller
	# specified an explicit test set then instead we should just stick to one.
	if [ "${#tests_to_run[@]}" -eq 0 ] && [[ "${GITHUB_ACTIONS:-}" == "true" ]]; then
		partition=2
	fi
}
# If --partition contains a slash then it indicates that we are running as
# a _specific_ partition. If it is just numeric then it is indicating the
# _number_ of partitions.
partitions=
if [[ "$partition" == *"/"* ]]; then
	partitions=1
	partition="hash:$partition"
else
	partitions="$partition"
	partition=
fi

[ "${#enosys_syscalls[@]}" -gt 0 ] && {
	cargo build --release --manifest-path "$SRC_ROOT/contrib/fake-enosys/Cargo.toml"
	FAKE_ENOSYS="$SRC_ROOT/target/release/fake-enosys"
	# Make sure the syscalls are valid.
	"$FAKE_ENOSYS" -s "$(strjoin , "${enosys_syscalls[@]}")" true || \
		bail "--enosys=$(strjoin , "${enosys_syscalls[@]}") contains invalid syscalls"
}

function llvm-profdata() {
	local profdata

	{ command llvm-profdata --help &>/dev/null && profdata=llvm-profdata ; } ||
		{ command rust-profdata --help &>/dev/null && profdata=rust-profdata ; } ||
		{ command cargo-profdata --help &>/dev/null && profdata=cargo-profdata ; } ||
		bail "cannot find llvm-profdata!"

	command "$profdata" "$@"
}

function merge_llvmcov_profdata() {
	local llvmcov_targetdir=target/llvm-cov-target

	# Get a list of *.profraw files for merging.
	local profraw_list
	profraw_list="$(mktemp --tmpdir libpathrs-profraw.XXXXXXXX)"
	find "$llvmcov_targetdir" -name '*.profraw' -type f >"$profraw_list"
	#shellcheck disable=SC2064
	trap "rm -f '$profraw_list'; trap - RETURN" RETURN

	# Merge profiling data. This is what cargo-llvm-cov does internally, and
	# they also make use of --sparse to remove useless entries.
	local combined_profraw
	combined_profraw="$(mktemp libpathrs-combined-profraw.XXXXXXXX)"
	llvm-profdata merge --sparse -f "$profraw_list" -o "$combined_profraw"

	# Remove the old profiling data and replace it with the merged version. As
	# long as the file has a ".profraw" suffix, cargo-llvm-cov will use it.
	find "$llvmcov_targetdir" -name '*.profraw' -type f -delete
	mv "$combined_profraw" "$llvmcov_targetdir/libpathrs-combined.profraw"
}

function nextest_run() {
	local features=("capi")

	# Add any extra features passed in the environment.
	local extra extra_features
	IFS=, read -ra extra_features <<<"${EXTRA_FEATURES:-}"
	for extra in "${extra_features[@]}"; do
		[ -n "$extra" ] && features+=("$extra")
	done

	if [ -v CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER ]; then
		unset CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER
	fi

	if [ -n "$sudo" ]; then
		features+=("_test_as_root")

		# This CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER magic lets us run
		# Rust tests as root without needing to run the build step as root.
		export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER="sudo -E "
	fi

	build_args=()
	if [ "${#features[@]}" -gt 0 ]; then
		build_args=("--workspace" "--features" "$(strjoin , "${features[@]}")")
	fi

	if [ "${#enosys_syscalls[@]}" -gt 0 ]; then
		export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER
		CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER+="$FAKE_ENOSYS -s $(strjoin , "${enosys_syscalls[@]}") -- "
	fi

	archive_args=()
	if [ -n "$nextest_archive" ]; then
		archive_args+=(--archive-file "$nextest_archive" --workspace-remap "$SRC_ROOT")
		# When using --archive-file we cannot set --features.
		echo "features '${features[*]}' are ignored with --archive-file" >&2
		build_args=()
	fi

	for partnum in $(seq "$partitions"); do
		part="${partition:-hash:$partnum/$partitions}"

		if [ -n "$nextest_archive" ]; then
			# When using --archive-file, using the same target dir for multiple
			# "cargo llvm-cov nextest run" instances causes errors:
			#   error: error extracting archive `./pathrs.tar.zst`
			#   destination `/home/cyphar/src/libpathrs/target/llvm-cov-target/target` already exists
			rm -rf "$SRC_ROOT/target/llvm-cov-target/target"
		fi

		$CARGO \
			llvm-cov --no-report --branch "${build_args[@]}" \
			nextest --partition="$part" "${archive_args[@]}" "$@"

		# It turns out that a very large amount of diskspace gets used up by
		# the thousands of tiny .profraw files generated during each
		# integration test run (up to ~22GB in our GHA CI).
		#
		# This can cause disk exhaustion (and thus CI failures), so we need to
		# proactively merge the profiling data to reduce its size (in addition
		# to running the tests in partitions to avoid accumulating all 22GBs of
		# profiling data before merging).
		#
		# cargo-llvm-cov will happily accept these merged files, and this kind
		# of merging is what it does internally anyway.
		merge_llvmcov_profdata
	done

	if [ -v CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER ]; then
		unset CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER
	fi
}

set -x

# Increase the maximum file descriptor limit from the default 1024 to whatever
# the hard limit is (which should be large enough) so that our racing
# remove_all tests won't fail with EMFILE. Ideally this workaround wouldn't be
# necessary, see <https://github.com/cyphar/libpathrs/issues/149>.
ulimit -n "$(ulimit -Hn)"

if [ "${#tests_to_run[@]}" -gt 0 ]; then
	for test_spec in "${tests_to_run[@]}"; do
		nextest_run --no-fail-fast -E "$test_spec"
	done
else
	# We need to run race and non-race tests separately because the racing
	# tests can cause the non-race tests to error out spuriously. Hopefully in
	# the future <https://github.com/nextest-rs/nextest/discussions/2054> will
	# be resolved and nextest will make it easier to do this.
	nextest_run --no-fail-fast -E "not test(#tests::test_race_*)"
	nextest_run --no-fail-fast -E "test(#tests::test_race_*)"
fi
