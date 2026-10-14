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
	"strings"
	"syscall"

	"github.com/urfave/cli/v3"
	"golang.org/x/sys/unix"

	"cyphar.com/go-pathrs"
)

var versionCmd = &cli.Command{
	Name:  "version",
	Usage: "output the libpathrs.so version",
	Action: func(_ context.Context, _ *cli.Command) error {
		version, err := pathrs.LibraryVersion()
		if err != nil {
			return err
		}
		fmt.Printf("VERSION %s\n", version.VersionString)
		return nil
	},
}

// unescapeArg decodes the escape sequences in a command-line argument for
// PATHRS_CMD_UNESCAPE=1.
func unescapeArg(arg string) (string, error) {
	var unescaped strings.Builder
	for i := 0; i < len(arg); i++ {
		if arg[i] != '\\' {
			unescaped.WriteByte(arg[i])
			continue
		}
		i++
		if i >= len(arg) {
			return "", fmt.Errorf("trailing unescaped '\\' in %q", arg)
		}
		// Only "\\" and "\0" are allowed.
		switch arg[i] {
		case '0':
			unescaped.WriteByte(0)
		case '\\':
			unescaped.WriteByte('\\')
		default:
			return "", fmt.Errorf("invalid escape sequence '\\%c' in %q", arg[i], arg)
		}
	}
	return unescaped.String(), nil
}

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

	if os.Getenv("PATHRS_CMD_UNESCAPE") != "" {
		// We must skip argv[0].
		for i := 1; i < len(args); i++ {
			arg, err := unescapeArg(args[i])
			if err != nil {
				return fmt.Errorf("unescape argument (PATHRS_CMD_UNESCAPE is set): %w", err)
			}
			args[i] = arg
		}
	}

	cmd := &cli.Command{
		Name:  "pathrs-cmd",
		Usage: "helper binary for testing libpathrs",
		Authors: []any{
			"Aleksa Sarai <cyphar@cyphar.com>",
		},
		Commands: []*cli.Command{
			versionCmd,
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
