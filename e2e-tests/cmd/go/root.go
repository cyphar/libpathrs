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

	"github.com/urfave/cli/v3"
	"golang.org/x/sys/unix"

	"cyphar.com/go-pathrs"
)

var rootCmd = &cli.Command{
	Name:  "root",
	Usage: "Root::* operations",
	Flags: []cli.Flag{
		&cli.StringFlag{
			Name:     "root",
			Required: true,
		},
	},
	Commands: []*cli.Command{
		rootResolveCmd,
		rootOpenCmd,
		rootMkfileCmd,
		rootMkdirCmd,
		rootMkdirAllCmd,
		rootMknodCmd,
		rootHardlinkCmd,
		rootSymlinkCmd,
		rootReadlinkCmd,
		rootUnlinkCmd,
		rootRmdirCmd,
		rootRmdirAllCmd,
		rootRenameCmd,
	},

	Before: func(ctx context.Context, cmd *cli.Command) (context.Context, error) {
		rootPath := cmd.String("root")
		// The "required" flag checks in urfave/cli happen after Before is run,
		// so we need to manually check this here.
		if rootPath == "" {
			return nil, errors.New(`Required flag "root" not set`)
		}

		root, err := pathrs.OpenRoot(rootPath)
		if err != nil {
			return nil, err
		}
		ctx = context.WithValue(ctx, "root", root)
		return ctx, nil
	},

	After: func(ctx context.Context, cmd *cli.Command) error {
		if root, ok := ctx.Value("root").(*pathrs.Root); ok {
			_ = root.Close()
		}
		return nil
	},
}

var rootResolveCmd = cmdWithOptions(&cli.Command{
	Name:  "resolve",
	Usage: "resolve a path inside the root",
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
		root := ctx.Value("root").(*pathrs.Root)
		follow := cmd.Bool("follow")
		subpath := cmd.StringArg("subpath")

		var (
			handle *pathrs.Handle
			err    error
		)
		if follow {
			handle, err = root.Resolve(subpath)
		} else {
			handle, err = root.ResolveNoFollow(subpath)
		}
		if err != nil {
			return err
		}
		defer handle.Close()

		fmt.Println("HANDLE-PATH", handle.IntoFile().Name())

		if val := ctx.Value("reopen"); val != nil {
			oflags := val.(int)
			f, err := handle.OpenFile(oflags)
			if err != nil {
				return err
			}
			// TODO: Input/output file data.
			fmt.Println("FILE-PATH", f.Name())
		}

		return nil
	},
}, oflags("reopen", "reopen the handle with these O_* flags", nil))

var rootOpenCmd = cmdWithOptions(&cli.Command{
	Name:  "open",
	Usage: "open a path inside the root",
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
		root := ctx.Value("root").(*pathrs.Root)
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
		if oflags == 0 /* O_RDONLY */ {
			f, err = root.Open(subpath)
		} else {
			f, err = root.OpenFile(subpath, oflags)
		}
		if err != nil {
			return err
		}
		defer f.Close()

		// TODO: Input/output file data.
		fmt.Println("FILE-PATH", f.Name())
		return nil
	},
}, oflags("oflags", "O_* flags to use when opening the file", unix.O_RDONLY))

var rootMkfileCmd = cmdWithOptions(&cli.Command{
	Name:  "mkfile",
	Usage: "make an empty file inside the root",
	Arguments: []cli.Argument{
		&cli.StringArg{
			Name: "subpath",
		},
	},

	Action: func(ctx context.Context, cmd *cli.Command) error {
		root := ctx.Value("root").(*pathrs.Root)
		subpath := cmd.StringArg("subpath")
		oflags := ctx.Value("oflags").(int)
		mode := ctx.Value("mode").(os.FileMode)

		f, err := root.Create(subpath, oflags, mode)
		if err != nil {
			return err
		}
		defer f.Close()

		// TODO: Input/output file data?
		fmt.Println("FILE-PATH", f.Name())
		return nil
	},
},
	oflags("oflags", "O_* flags to use when creating the file", unix.O_RDONLY),
	modeFlag("mode", "file mode for the created file", "0o644"),
)

var rootMkdirCmd = cmdWithOptions(&cli.Command{
	Name:  "mkdir",
	Usage: "make an empty directory inside the root",
	Arguments: []cli.Argument{
		&cli.StringArg{
			Name: "subpath",
		},
	},

	Action: func(ctx context.Context, cmd *cli.Command) error {
		root := ctx.Value("root").(*pathrs.Root)
		subpath := cmd.StringArg("subpath")
		mode := ctx.Value("mode").(os.FileMode)

		return root.Mkdir(subpath, mode)
	},
},
	modeFlag("mode", "file mode for the created directory", "0o755"),
)

var rootMkdirAllCmd = cmdWithOptions(&cli.Command{
	Name:  "mkdir-all",
	Usage: "make a directory (including parents) inside the root",
	Arguments: []cli.Argument{
		&cli.StringArg{
			Name: "subpath",
		},
	},

	Action: func(ctx context.Context, cmd *cli.Command) error {
		root := ctx.Value("root").(*pathrs.Root)
		subpath := cmd.StringArg("subpath")
		mode := ctx.Value("mode").(os.FileMode)

		handle, err := root.MkdirAll(subpath, mode)
		if err != nil {
			return err
		}
		defer handle.Close()

		fmt.Println("HANDLE-PATH", handle.IntoFile().Name())
		return nil
	},
},
	modeFlag("mode", "file mode for the created directories", "0o755"),
)

var rootMknodCmd = cmdWithOptions(&cli.Command{
	Name:  "mknod",
	Usage: "make an inode inside the root",
	Arguments: []cli.Argument{
		&cli.StringArg{
			Name: "subpath",
		},
		&cli.StringArg{
			Name: "type",
		},
		&cli.Uint32Arg{
			Name: "major",
		},
		&cli.Uint32Arg{
			Name: "minor",
		},
	},

	Action: func(ctx context.Context, cmd *cli.Command) error {
		root := ctx.Value("root").(*pathrs.Root)
		subpath := cmd.StringArg("subpath")
		mode := ctx.Value("mode").(os.FileMode)

		inoType := cmd.StringArg("type")
		switch inoType {
		case "":
			return fmt.Errorf("type is a required positional argument")
		case "f":
			// no-op
		case "d":
			mode |= os.ModeDir
		case "b":
			mode |= os.ModeDevice
		case "c", "u":
			mode |= os.ModeCharDevice | os.ModeDevice
		case "p":
			mode |= os.ModeNamedPipe
		default:
			return fmt.Errorf("unknown type %s", inoType)
		}
		dev := unix.Mkdev(cmd.Uint32Arg("major"), cmd.Uint32Arg("minor"))

		return root.Mknod(subpath, mode, dev)
	},
},
	modeFlag("mode", "file mode for the created inode", "0o644"),
)

var rootHardlinkCmd = &cli.Command{
	Name:  "hardlink",
	Usage: "make a hardlink inside the root",
	Arguments: []cli.Argument{
		&cli.StringArg{
			Name: "target",
		},
		&cli.StringArg{
			Name: "linkname",
		},
	},

	Action: func(ctx context.Context, cmd *cli.Command) error {
		root := ctx.Value("root").(*pathrs.Root)
		target := cmd.StringArg("target")
		linkname := cmd.StringArg("linkname")

		return root.Hardlink(target, linkname)
	},
}

var rootSymlinkCmd = &cli.Command{
	Name:  "symlink",
	Usage: "make a symbolic link inside the root",
	Arguments: []cli.Argument{
		&cli.StringArg{
			Name: "target",
		},
		&cli.StringArg{
			Name: "linkname",
		},
	},

	Action: func(ctx context.Context, cmd *cli.Command) error {
		root := ctx.Value("root").(*pathrs.Root)
		target := cmd.StringArg("target")
		linkname := cmd.StringArg("linkname")

		return root.Symlink(target, linkname)
	},
}

var rootReadlinkCmd = &cli.Command{
	Name:  "readlink",
	Usage: "read the target path of a symbolic link inside the root",
	Arguments: []cli.Argument{
		&cli.StringArg{
			Name: "subpath",
		},
	},

	Action: func(ctx context.Context, cmd *cli.Command) error {
		root := ctx.Value("root").(*pathrs.Root)
		subpath := cmd.StringArg("subpath")

		target, err := root.Readlink(subpath)
		if err != nil {
			return err
		}
		fmt.Println("LINK-TARGET", target)
		return nil
	},
}

var rootUnlinkCmd = &cli.Command{
	Name:  "unlink",
	Usage: "remove a file inside the root",
	Arguments: []cli.Argument{
		&cli.StringArg{
			Name: "subpath",
		},
	},

	Action: func(ctx context.Context, cmd *cli.Command) error {
		root := ctx.Value("root").(*pathrs.Root)
		subpath := cmd.StringArg("subpath")

		return root.RemoveFile(subpath)
	},
}

var rootRmdirCmd = &cli.Command{
	Name:  "rmdir",
	Usage: "remove an (empty) directory inside the root",
	Arguments: []cli.Argument{
		&cli.StringArg{
			Name: "subpath",
		},
	},

	Action: func(ctx context.Context, cmd *cli.Command) error {
		root := ctx.Value("root").(*pathrs.Root)
		subpath := cmd.StringArg("subpath")

		return root.RemoveDir(subpath)
	},
}

var rootRmdirAllCmd = &cli.Command{
	Name:  "rmdir-all",
	Usage: "remove a path (recursively) inside the root",
	Arguments: []cli.Argument{
		&cli.StringArg{
			Name: "subpath",
		},
	},

	Action: func(ctx context.Context, cmd *cli.Command) error {
		root := ctx.Value("root").(*pathrs.Root)
		subpath := cmd.StringArg("subpath")

		return root.RemoveAll(subpath)
	},
}

var rootRenameCmd = &cli.Command{
	Name:  "rename",
	Usage: "rename a path inside the root",
	Flags: []cli.Flag{
		&cli.BoolFlag{
			Name: "exchange",
		},
		&cli.BoolFlag{
			Name: "whiteout",
		},
		&cli.BoolWithInverseFlag{
			Name:  "clobber",
			Value: true,
		},
	},
	Arguments: []cli.Argument{
		&cli.StringArg{
			Name: "source",
		},
		&cli.StringArg{
			Name: "destination",
		},
	},

	Action: func(ctx context.Context, cmd *cli.Command) error {
		root := ctx.Value("root").(*pathrs.Root)
		src := cmd.StringArg("source")
		dst := cmd.StringArg("destination")

		var renameArgs uint
		if !cmd.Bool("clobber") {
			renameArgs |= unix.RENAME_NOREPLACE
		}
		if cmd.Bool("exchange") {
			renameArgs |= unix.RENAME_EXCHANGE
		}
		if cmd.Bool("whiteout") {
			renameArgs |= unix.RENAME_WHITEOUT
		}
		return root.Rename(src, dst, renameArgs)
	},
}
