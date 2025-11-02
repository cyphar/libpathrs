#!/usr/bin/jq -f
# SPDX-License-Identifier: MPL-2.0
#
# libpathrs: safe path resolution on Linux
# Copyright (C) 2019-2025 Aleksa Sarai <cyphar@cyphar.com>
# Copyright (C) 2019-2025 SUSE LLC
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

# Compute the inverse set of the filters provided to this script. This script
# is used by .github/workflows/rust.yml to partition tests into logical blocks
# without accidentally missing any tests.

def compute_misc($arr):
	$arr + [
		{
			"name": "misc",
			"pattern": "not (\($arr | map("(" + .pattern + ")") | join(" or ")))"
		}
	]
;

# Use a sane default if nothing is specified.
compute_misc(. // [
	{
		"name": "Root",
		"pattern": "test(#tests::test_root_ops::root_*) or test(#root::*)"
	},
	{
		"name": "RootRef",
		"pattern": "test(#tests::test_root_ops::rootref_*) or test(#tests::test_root_ops::capi_*)"
	},
	{
		"name": "procfs",
		"pattern": "test(#tests::test_procfs*) or test(*proc*)"
	},
	{
		"name": "resolver",
		"pattern": "test(#tests::test_resolve*) or test(#resolvers::*)"
	},
	{
		"name": "race",
		"pattern": "test(#tests::test_race*)"
	}
])
