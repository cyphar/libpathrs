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
	"fmt"
	"os"
	"strconv"
	"strings"

	"github.com/urfave/cli/v3"
	"golang.org/x/sys/unix"

	"cyphar.com/go-pathrs/procfs"
)

type procfsOpenFunc func(path string, flags int) (*os.File, error)

var procfsCmd = &cli.Command{
	Name:  "procfs",
	Usage: "ProcfsHandle::* operations",
	Flags: []cli.Flag{
		&cli.BoolFlag{
			Name:  "unmasked",
			Usage: "use unmasked procfs handle",
		},
		&cli.StringFlag{
			Name:  "base",
			Usage: "base path for procfs operations (root, pid=<n>, self, thread-self)",
			Value: "root",
		},
	},
	Commands: []*cli.Command{
		procfsOpenCmd,
		procfsReadlinkCmd,
	},

	Before: func(ctx context.Context, cmd *cli.Command) (_ context.Context, Err error) {
		var opts []procfs.OpenOption
		if cmd.Bool("unmasked") {
			opts = append(opts, procfs.UnmaskedProcRoot)
		}
		proc, err := procfs.Open(opts...)
		if err != nil {
			return nil, err
		}
		ctx = context.WithValue(ctx, "procfs", proc)
		defer func() {
			if Err != nil {
				_ = proc.Close()
			}
		}()

		return ctx, nil
	},

	After: func(ctx context.Context, cmd *cli.Command) error {
		if proc, ok := ctx.Value("procfs").(*procfs.Handle); ok {
			_ = proc.Close()
		}
		return nil
	},
}

var procfsOpenCmd = cmdWithOptions(&cli.Command{
	Name:  "open",
	Usage: "open a path in procfs",
	Flags: []cli.Flag{
		&cli.BoolWithInverseFlag{
			Name:  "follow",
			Usage: "follow trailing symlinks",
			Value: true,
		},
	},
	Arguments: []cli.Argument{
		&cli.StringArg{
			Name: "subpath",
		},
	},

	Action: func(ctx context.Context, cmd *cli.Command) error {
		proc := ctx.Value("procfs").(*procfs.Handle)
		follow := cmd.Bool("follow")
		subpath := cmd.StringArg("subpath")

		oflags := unix.O_RDONLY
		if val := ctx.Value("oflags"); val != nil {
			oflags = val.(int)
		}
		if !follow {
			oflags |= unix.O_NOFOLLOW
		}

		var (
			f   *os.File
			err error
		)

		baseStr := cmd.String("base")
		if pidStr, ok := strings.CutPrefix(baseStr, "pid="); ok {
			var pid int
			pid, err = strconv.Atoi(pidStr)
			if err != nil {
				return fmt.Errorf("failed to parse --base=%q: %w", pidStr, err)
			}
			f, err = proc.OpenPid(pid, subpath, oflags)
		} else {
			switch baseStr {
			case "root":
				f, err = proc.OpenRoot(subpath, oflags)
			case "self":
				f, err = proc.OpenSelf(subpath, oflags)
			case "thread-self":
				var closer procfs.ThreadCloser
				f, closer, err = proc.OpenThreadSelf(subpath, oflags)
				if closer != nil {
					defer closer()
				}
			default:
				return fmt.Errorf("invalid --base value %q", baseStr)
			}
		}
		if err != nil {
			return err
		}
		defer f.Close()

		fmt.Println("FILE-PATH", f.Name())
		// TODO: Input/output file data.
		return nil
	},
}, oflags("oflags", "O_* flags to use when opening the file", unix.O_RDONLY))

var procfsReadlinkCmd = &cli.Command{
	Name:  "readlink",
	Usage: "read the target path of a symbolic link in procfs",
	Arguments: []cli.Argument{
		&cli.StringArg{
			Name: "subpath",
		},
	},

	Action: func(ctx context.Context, cmd *cli.Command) error {
		proc := ctx.Value("procfs").(*procfs.Handle)
		subpath := cmd.StringArg("subpath")

		var base procfs.ProcBase
		baseStr := cmd.String("base")
		if pidStr, ok := strings.CutPrefix(baseStr, "pid="); ok {
			pid, err := strconv.Atoi(pidStr)
			if err != nil {
				return fmt.Errorf("failed to parse --base=%q: %w", pidStr, err)
			}
			base = procfs.ProcPid(pid)
		} else {
			switch baseStr {
			case "root":
				base = procfs.ProcRoot
			case "self":
				base = procfs.ProcSelf
			case "thread-self":
				base = procfs.ProcThreadSelf
			default:
				return fmt.Errorf("invalid --base value %q", baseStr)
			}
		}

		target, err := proc.Readlink(base, subpath)
		if err != nil {
			return err
		}
		fmt.Println("LINK-TARGET", target)
		return nil
	},
}
