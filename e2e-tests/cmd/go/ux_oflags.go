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
	"fmt"
	"strings"

	"github.com/urfave/cli/v3"
	"golang.org/x/sys/unix"
)

var oflagValues = map[string]int{
	// Access modes (including O_PATH).
	"O_RDWR":   unix.O_RDWR,
	"O_RDONLY": unix.O_RDONLY,
	"O_WRONLY": unix.O_WRONLY,
	"O_PATH":   unix.O_PATH,
	// Fd flags.
	"O_CLOEXEC": unix.O_CLOEXEC,
	// Control lookups.
	"O_NOFOLLOW":  unix.O_NOFOLLOW,
	"O_DIRECTORY": unix.O_DIRECTORY,
	"O_NOCTTY":    unix.O_NOCTTY,
	// NOTE: This flag contains O_DIRECTORY!
	"O_TMPFILE": unix.O_TMPFILE,
	// File creation.
	"O_CREAT":  unix.O_CREAT,
	"O_EXCL":   unix.O_EXCL,
	"O_TRUNC":  unix.O_TRUNC,
	"O_APPEND": unix.O_APPEND,
	// Sync.
	"O_SYNC":     unix.O_SYNC,
	"O_ASYNC":    unix.O_ASYNC,
	"O_DSYNC":    unix.O_DSYNC,
	"O_FSYNC":    unix.O_FSYNC,
	"O_RSYNC":    unix.O_RSYNC,
	"O_DIRECT":   unix.O_DIRECT,
	"O_NDELAY":   unix.O_NDELAY,
	"O_NOATIME":  unix.O_NOATIME,
	"O_NONBLOCK": unix.O_NONBLOCK,
}

func parseOflags(flags string) (int, error) {
	oflagFieldsFunc := func(ch rune) bool {
		return ch == '|' || ch == ','
	}

	var oflags int
	for flag := range strings.FieldsFuncSeq(flags, oflagFieldsFunc) {
		// Convert any flags to -> O_*.
		flag = strings.ToUpper(flag)
		if !strings.HasPrefix(flag, "O_") {
			flag = "O_" + flag
		}
		val, ok := oflagValues[flag]
		if !ok {
			return 0, fmt.Errorf("unknown flag name %q", flag)
		}
		oflags |= val
	}
	return oflags, nil
}

func oflags(name, usage string, dfl any) uxOption {
	return func(cmd *cli.Command) *cli.Command {
		cmd.Flags = append(cmd.Flags, &cli.StringFlag{
			Name:  name,
			Usage: usage + " (comma- or |-separated)",
		})

		// TODO: Should we wrap Action instead?
		oldBefore := cmd.Before
		cmd.Before = func(ctx context.Context, cmd *cli.Command) (context.Context, error) {
			if cmd.IsSet(name) {
				oflags, err := parseOflags(cmd.String(name))
				if err != nil {
					return nil, fmt.Errorf("error parsing --%s: %w", name, err)
				}
				ctx = context.WithValue(ctx, name, oflags)
			} else {
				ctx = context.WithValue(ctx, name, dfl)
			}
			var err error
			if oldBefore != nil {
				ctx, err = oldBefore(ctx, cmd)
			}
			return ctx, err
		}

		return cmd
	}
}
