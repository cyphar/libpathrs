#!/bin/bash
# SPDX-License-Identifier: MPL-2.0 OR LGPL-3.0-or-later
#
# libpathrs: safe path resolution on Linux
# Copyright (C) 2019-2025 SUSE LLC
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

# check-elf-symbols.sh -- make sure that libpathrs.so only contains pathrs_*
# symbols with versions included, to ensure we do not regress our symbol
# versioning again.

set -Eeuo pipefail

SRC_ROOT="$(readlink -f "$(dirname "${BASH_SOURCE[0]}")/..")"
ELF_FILE="$1"

function bail() {
	echo "ERROR:" "$@" >&2
	exit 1
}

# TODO: Do we actually want to also check the nodes referenced in src/capi/*?
# TODO: Also, once we make the old symbols optional this script will break.
readarray -t EXPECTED_VERSION_NODES < <(
	grep -Eo "LIBPATHRS_[0-9]+(\.[0-9]+)+" "$SRC_ROOT/build.rs" | sort -u
)

# Make sure that we at least have LIBPATHRS_0.[12].
for node in LIBPATHRS_0.{1,2}; do
	printf "%s\n" "${EXPECTED_VERSION_NODES[@]}" | grep -Fxq "$node" \
		|| bail "$node node not found in build.rs"
done

echo "== ELF SYMBOL VERSION NODES =="

for node in "${EXPECTED_VERSION_NODES[@]}"; do
	readelf -WV "$ELF_FILE" | grep -Fwq "$node" || \
		bail "$node node not found in $ELF_FILE"
	echo "$node node present in $ELF_FILE"
done

echo "== SYMBOL VERSIONS SUMMARY =="

SYMBOL_OBJDUMP="$(objdump -T "$ELF_FILE" | grep -F "pathrs_")"

[ -n "$SYMBOL_OBJDUMP" ] || bail "$ELF_FILE does not contain any pathrs_* symbols"
echo "$(wc -l <<<"$SYMBOL_OBJDUMP") pathrs symbols present in $ELF_FILE"

for node in "${EXPECTED_VERSION_NODES[@]}"; do
	awk -v NODE="$node" '
	BEGIN { num_symbols = 0 }
	$(NF-1) == NODE {
		symbols[$NF]++
		num_symbols++
	}
	END {
		print NODE, "symbols (" num_symbols, "total):"
		if (num_symbols) {
			for (sym in symbols) {
				print "  " sym
			}
		}
	}
	' <<<"$SYMBOL_OBJDUMP"
done

unversioned="$(awk '!($(NF-1) ~ /^LIBPATHRS_/)' <<<"$SYMBOL_OBJDUMP")"
[ -z "$unversioned" ] || {
	echo "UNVERSIONED SYMBOLS ($(wc -l <<<"$unversioned") total):"
	echo "$unversioned"
	bail "$ELF_FILE contains unversioned symbols"
}

echo "++ ALL SYMBOLS ARE VERSIONED! ++"
