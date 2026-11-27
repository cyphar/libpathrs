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

# Run a command as root with sudo, preserving the entire environment of the
# caller except for unsafe environment variables (similar to "sudo -E"). This
# is needed because sudo-rs (which Ubuntu 25.10) does not implement this
# behaviour, which breaks our tests.

set -Eeuo pipefail

# These are glob patterns for environment variables that "sudo -E" would drop,
# which need to be filtered before passing an environment list (--preserve-env
# does not do any filtering when explicitly listing them).
#
# Note that glibc also clears a bunch of dangerous environment variables during
# initialisation if running as setuid (AT_SECURE) before sudo or sudo-rs even
# see them.
bad_envvars=(
	# Emulate sudo's secure_path.
	PATH
	# Modern bash function exports.
	# (Technically a no-op since compgen won't output them in the first place.)
	"BASH_FUNC_*"
	# The rest come from initial_badenv_table in sudo's plugins/sudoers/env.c.
	IFS CDPATH LOCALDOMAIN RES_OPTIONS HOSTALIASES NLSPATH PATH_LOCALE
	"LD_*" "_RLD*"
	TERMINFO TERMINFO_DIRS TERMPATH TERMCAP
	ENV BASH_ENV PS4 GLOBIGNORE BASHOPTS SHELLOPTS
	JAVA_TOOL_OPTIONS _JAVA_OPTIONS CLASSPATH
	PERLIO_DEBUG PERLLIB PERL5LIB PERL5OPT PERL5DB
	FPATH NULLCMD READNULLCMD ZDOTDIR TMPPREFIX
	PYTHONHOME PYTHONPATH PYTHONINSPECT PYTHONUSERBASE PYTHONSTARTUP
	RUBYLIB RUBYOPT NODE_OPTIONS NODE_PATH
	GIT_SSH_COMMAND "GIT_CONFIG_*"
	GCONV_PATH
)

function is_excluded() {
	local name="$1" pattern
	# Block bash function exports. sudo blocks them with a "*=()*" glob.
	[[ "${!name}" == "()"* ]] && return 0
	for pattern in "${bad_envvars[@]}"; do
		# We want to use the pattern as a glob.
		#shellcheck disable=SC2053
		[[ "$name" == $pattern ]] && return 0
	done
	return 1
}

preserved_envvars=()
while read -r name; do
	is_excluded "$name" || preserved_envvars+=("$name")
done < <(compgen -e)

exec sudo --preserve-env="$(IFS=,; echo "${preserved_envvars[*]}")" -- "$@"
