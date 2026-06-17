#!/bin/bash
# SPDX-License-Identifier: MPL-2.0
#
# libpathrs: safe path resolution on Linux
# Copyright (C) 2026 Aleksa Sarai <cyphar@cyphar.com>
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

set -Eeuxo pipefail

SRC_ROOT="$(readlink -f "$(dirname "${BASH_SOURCE[0]}")/..")"
TAG="$1"

# Make sure that the tag is actually signed by the maintainer (me).
# TODO: Support checking the exact email addresses in libpathrs.keyring.
git show -- "$TAG" | grep -Fx "Author: Aleksa Sarai <cyphar@cyphar.com>"
git show -- "$TAG" | grep -Fx "Tagger: Aleksa Sarai <cyphar@cyphar.com>"

# Use a temporary GNUPGHOME home to make sure we are only permitting
# libpathrs-approved keys for signing the release commit.
export GNUPGHOME="$(mkdir -d --tmpdir gpg-home-libpathrs.XXXXXXX)"
trap 'rm -rf "$GNUPGHOME"' EXIT
gpg --import <"$SRC_ROOT/libpathrs.keyring"

git verify-tag -- "$TAG"
git tag -v -- "$TAG"
