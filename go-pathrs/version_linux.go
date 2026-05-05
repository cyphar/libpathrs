//go:build linux

// SPDX-License-Identifier: MPL-2.0
/*
 * libpathrs: safe path resolution on Linux
 * Copyright (C) 2019-2025 SUSE LLC
 * Copyright (C) 2026 Aleksa Sarai <cyphar@cyphar.com>
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

package pathrs

import (
	"cyphar.com/go-pathrs/internal/libpathrs"
)

// Version returns the version string of the underlying libpathrs C library
// that go-pathrs is linked against.
//
// The returned string follows [SemVer 2.0.0] (with an optional
// "+<build-metadata>" suffix for pre-release builds, e.g. "0.2.4+dev").
//
// [SemVer 2.0.0]: https://semver.org/spec/v2.0.0.html
func Version() string {
	return libpathrs.Version()
}
