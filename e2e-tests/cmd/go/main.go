// SPDX-License-Identifier: MPL-2.0
/*
 * libpathrs: safe path resolution on Linux
 * Copyright (C) 2019-2025 Aleksa Sarai <cyphar@cyphar.com>
 * Copyright (C) 2019-2025 SUSE LLC
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

package main

import (
	"context"
	"errors"
	"fmt"
	"os"
	"syscall"

	"github.com/urfave/cli/v3"
)

func main() {
	cmd := &cli.Command{
		Name:  "pathrs-cmd",
		Usage: "helper binary for testing libpathrs",
		Authors: []any{
			"Aleksa Sarai <cyphar@cyphar.com>",
		},
		Commands: []*cli.Command{
			rootCmd,
			procfsCmd,
		},
	}
	if err := cmd.Run(context.Background(), os.Args); err != nil {
		var errno syscall.Errno
		if errors.As(err, &errno) {
			fmt.Fprintf(os.Stderr, "ERRNO %d (%s)\n", errno, errno)
		}
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}
}
