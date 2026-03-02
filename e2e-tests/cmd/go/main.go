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
	"context"
	"errors"
	"fmt"
	"os"
	"strconv"
	"syscall"

	"github.com/urfave/cli/v3"
	"golang.org/x/sys/unix"
)

func Main(args []string) error {
	if dumpableStr := os.Getenv("PR_SET_DUMPABLE"); dumpableStr != "" {
		dumpable, err := strconv.ParseUint(dumpableStr, 10, 64)
		if err != nil {
			return fmt.Errorf("invalid PR_SET_DUMPABLE environment variable value: %w", err)
		}
		if err := unix.Prctl(unix.PR_SET_DUMPABLE, uintptr(dumpable), 0, 0, 0); err != nil {
			return fmt.Errorf("failed to set PR_SET_DUMPABLE=%v: %w", dumpable, err)
		}
	}

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
	return cmd.Run(context.Background(), args)
}

func main() {
	if err := Main(os.Args); err != nil {
		var errno syscall.Errno
		if errors.As(err, &errno) {
			fmt.Fprintf(os.Stderr, "ERRNO %d (%s)\n", errno, errno)
		}
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}
}
