#!/bin/bash
# SPDX-License-Identifier: MPL-2.0 OR LGPL-3.0-or-later
#
# libpathrs: safe path resolution on Linux
# Copyright (C) 2026 Aleksa Sarai <cyphar@cyphar.com>
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

# cargo's --crate-type= argument was stabilised in Rust 1.64 but our MSRV is
# 1.63. This script emulates --crate-type= by temporarily modifying the
# crate-type option in Cargo.toml.

set -Eeuo pipefail

bail() {
	echo "$*" >&2
	exit 1
}

SRC_ROOT="$(readlink -f "$(dirname "${BASH_SOURCE[0]}")/..")"

# Only parse the first argument if it is --crate-type=foo. The rest of the
# arguments get passed as-is.
crate_types=("rlib" "staticlib" "cdylib")
case "${1:-}" in
	--crate-type=*)
		arg="${1#*=}"
		IFS=', ' read -r -a crate_types <<<"$arg"
		shift 1
		;;
esac

[ "$#" -gt 0 ] || bail "usage: with-crate-type.sh [--crate-type=...] <command> [...]"

set -x

# Make a backup of Cargo.toml and Cargo.lock. The lockfile backup is needed
# because dropping members from [workspace] causes cargo to prune their
# dependencies from the lockfile, which is not what we want.
backup_dir="$(mktemp -d "$SRC_ROOT/.cargo-backup.XXXXXX")"
cp "$SRC_ROOT"/Cargo.{toml,lock} "$backup_dir"
# shellcheck disable=SC2064 # We want to expand the variables immediately.
trap "mv -t '$SRC_ROOT/' -- '$backup_dir'/Cargo.*" EXIT

# Replace the crate-type.
sed -i \
	"/^crate-type/ s/=.*/= [$(printf '"%s",' "${crate_types[@]}")]/" \
	"$SRC_ROOT/Cargo.toml"

# Drop the workspace = [...] set. Some of our workspace crates have
# dependencies that use edition2024 which triggers a parsing error even if you
# do not actually build them, which causes problems for older distros.
# MSRV(1.85): Drop this once we require edition2024.
#
# TODO: If we ever split out libpathrs into subcrates we will need to cleverer.
#
# The sed-foo below collects everything from "members = [" until the next "]"
# into the pattern space and drops them in one go, to handle multi-line arrays
# more robustly.
sed -i '
	/^\[workspace\]/,/^\[/ {
		/^\s*members\s*=\s*\[/ { :x; /\]/!{ N; bx }; d }
	}' "$SRC_ROOT/Cargo.toml"

"$@"
