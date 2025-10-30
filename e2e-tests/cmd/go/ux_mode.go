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
	"os"
	"strconv"
	"strings"

	"github.com/urfave/cli/v3"
	"golang.org/x/sys/unix"
)

func parseModeFlag(modeStr string) (os.FileMode, error) {
	modeStr = strings.TrimPrefix(modeStr, "0o")

	unixMode, err := strconv.ParseUint(modeStr, 8, 32)
	if err != nil {
		return 0, fmt.Errorf("failed to parse --mode: %w", err)
	}
	if unixMode&^0o7777 != 0 {
		return 0, fmt.Errorf("invalid --mode %#o: must be subset of 0o7777")
	}

	mode := os.FileMode(unixMode & 0o777)
	if unixMode&unix.S_ISUID == unix.S_ISUID {
		mode |= os.ModeSetuid
	}
	if unixMode&unix.S_ISGID == unix.S_ISGID {
		mode |= os.ModeSetgid
	}
	if unixMode&unix.S_ISVTX == unix.S_ISVTX {
		mode |= os.ModeSticky
	}
	return mode, nil
}

func modeFlag(name, usage, dfl string) uxOption {
	return func(cmd *cli.Command) *cli.Command {
		cmd.Flags = append(cmd.Flags, &cli.StringFlag{
			Name:  name,
			Usage: usage,
			Value: dfl,
		})

		// TODO: Should we wrap Action instead?
		oldBefore := cmd.Before
		cmd.Before = func(ctx context.Context, cmd *cli.Command) (context.Context, error) {
			mode, err := parseModeFlag(cmd.String(name))
			if err != nil {
				return nil, fmt.Errorf("error parsing --%s: %w", name, err)
			}
			ctx = context.WithValue(ctx, name, mode)
			if oldBefore != nil {
				ctx, err = oldBefore(ctx, cmd)
			}
			return ctx, err
		}

		return cmd
	}
}
