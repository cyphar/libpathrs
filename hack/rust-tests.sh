#!/bin/bash
# SPDX-License-Identifier: LGPL-3.0-or-later
#
# libpathrs: safe path resolution on Linux
# Copyright (C) 2019-2025 Aleksa Sarai <cyphar@cyphar.com>
# Copyright (C) 2019-2025 SUSE LLC
#
# This program is free software: you can redistribute it and/or modify it under
# the terms of the GNU Lesser General Public License as published by the Free
# Software Foundation, either version 3 of the License, or (at your option) any
# later version.
#
# This program is distributed in the hope that it will be useful, but WITHOUT ANY
# WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A
# PARTICULAR PURPOSE. See the GNU General Public License for more details.
#
# You should have received a copy of the GNU Lesser General Public License along
# with this program. If not, see <https://www.gnu.org/licenses/>.

set -Eeuo pipefail

TEMP="$(getopt -o sc:p: --long sudo,cargo:,partitions: -- "$@")"
eval set -- "$TEMP"

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

sudo=
partitions=
CARGO="${CARGO_NIGHTLY:-cargo +nightly}"
while [ "$#" -gt 0 ]; do
	case "$1" in
		-s|--sudo)
			sudo=1
			shift
			;;
		-c|--cargo)
			CARGO="$2"
			shift 2
			;;
		-p|--partitions)
			partitions="$2"
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

[ -n "$partitions" ] || {
	partitions=1
	[[ "${GITHUB_ACTIONS:-}" == "true" ]] && partitions=2
}

# These are features that do not make sense to add to the powerset of feature
# combinations we test for:
# * "capi" only adds tests and modules in a purely additive way, so re-running
#   the whole suite without them makes no sense.
# * "_test_as_root" requires special handling to enable (the "sudo -E" runner).
SPECIAL_FEATURES=("capi" "_test_as_root")

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

		# This CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER magic lets us run Rust
		# tests as root without needing to run the build step as root.
		export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER="sudo -E"
	fi

	# For SPECIAL_FEATURES not explicitly set with --features, we need to add
	# them to --disabled-features to make sure that we don't add them to the
	# powerset.
	local disabled_features=()
	for feature in "${SPECIAL_FEATURES[@]}"; do
		if ! contains "$feature" "${features[@]}"; then
			disabled_features+=("$feature")
		fi
	done
	# By definition the default featureset is going to be included in
	# the powerset, so there's no need to duplicate it as well.
	disabled_features+=("default")

	local cargo_hack_args=()
	if command -v cargo-hack &>/dev/null ; then
		cargo_hack_args=(
			# Do a powerset run.
			"hack" "--feature-powerset"
			# With all disabled features (i.e. _test_as_root when not running
			# as root) dropped completely.
			"--exclude-features=$(strjoin , "${disabled_features[@]}")"
			# Also, since SPECIAL_FEATURES are all guaranteed to either be in
			# --features or --exclude-features, we do not need to do an
			# --all-features run with "cargo hack".
			"--exclude-all-features"
		)
	fi

	for partnum in $(seq "$partitions"); do
		# FIXME: Ideally we would use *nextest* partitioning, but because
		#        cargo-hack has a --partition flag we can only use that one.
		#        This should still resolve the issue but in a less granular
		#        way. See <https://github.com/taiki-e/cargo-hack/issues/286>.
		#part="hash:$partnum/$partitions"
		part="$partnum/$partitions"

		$CARGO "${cargo_hack_args[@]}" \
			llvm-cov --no-report --branch --features="$(strjoin , "${features[@]}")" \
			nextest --partition="$part" "$@"

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
	# "_test_enosys_statx" is not used with a whole-suite run because it would
	# just be wasteful -- we only set it for a smaller run of the procfs code.
	SPECIAL_FEATURES+=("_test_enosys_statx")

	# We need to run race and non-race tests separately because the racing
	# tests can cause the non-race tests to error out spuriously. Hopefully in
	# the future <https://github.com/nextest-rs/nextest/discussions/2054> will
	# be resolved and nextest will make it easier to do this.
	nextest_run --no-fail-fast -E "not test(#tests::test_race_*)"
	nextest_run --no-fail-fast -E "test(#tests::test_race_*)"

	# In order to avoid re-running the entire test suite with just statx
	# disabled, we re-run the key procfs tests with statx disabled.
	EXTRA_FEATURES=_test_enosys_statx \
		nextest_run --no-fail-fast -E "test(#tests::*procfs*)"
fi
