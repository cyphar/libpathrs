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

package main

import (
	"github.com/urfave/cli/v3"
)

// uxOption is an option that is applied to *[cli.Command] to modify its
// behaviour and add some new flag.
type uxOption func(*cli.Command) *cli.Command

func cmdWithOptions(cmd *cli.Command, opts ...uxOption) *cli.Command {
	for _, opt := range opts {
		cmd = opt(cmd)
	}
	return cmd
}
