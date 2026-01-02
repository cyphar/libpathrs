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

// Original author of this example code:
// Maxim Zhiburt <zhiburt@gmail.com> (2020)

// File: examples/c/cat.c
//
// An example program which opens a file inside a root and outputs its contents
// using libpathrs.

package main

import (
	"errors"
	"fmt"
	"io"
	"os"

	"cyphar.com/go-pathrs"
)

func Main(args ...string) error {
	if len(args) != 2 {
		fmt.Fprintln(os.Stderr, "usage: cat <root> <unsafe-path>")
		os.Exit(1)
	}

	rootPath, unsafePath := args[0], args[1]

	root, err := pathrs.OpenRoot(rootPath)
	if err != nil {
		return fmt.Errorf("open root %q: %w", rootPath, err)
	}
	defer root.Close()

	file, err := root.Open(unsafePath)
	if err != nil {
		return fmt.Errorf("open %q: %w", unsafePath, err)
	}
	defer file.Close()

	fmt.Fprintf(os.Stderr, "== file %q (from root %q) ==\n", file.Name(), root.IntoFile().Name())

	if _, err := io.Copy(os.Stdout, file); err != nil {
		return fmt.Errorf("copy file contents to stdout: %w", err)
	}
	return nil
}

func main() {
	if err := Main(os.Args[1:]...); err != nil {
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		fmt.Fprintf(os.Stderr, "Source: %v\n", errors.Unwrap(err))
		os.Exit(1)
	}
}
