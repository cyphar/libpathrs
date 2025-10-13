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
	"errors"
	"fmt"
	"io"
	"os"
	"strings"

	"golang.org/x/sys/unix"

	"github.com/cyphar/libpathrs/go-pathrs"
)

func Main(names ...string) error {
	proc, err := pathrs.OpenProcRoot(pathrs.UnmaskedProcRoot)
	if err != nil {
		return fmt.Errorf("open proc root: %w", err)
	}
	defer proc.Close()

	for _, name := range names {
		path := "sys/" + strings.ReplaceAll(name, ".", "/")

		file, err := proc.OpenRoot(path, unix.O_RDONLY)
		if err != nil {
			return fmt.Errorf("open sysctl %s: %w", name, err)
		}
		data, err := io.ReadAll(file)
		_ = file.Close()
		if err != nil {
			return fmt.Errorf("read sysctl %s: %w", name, err)
		}

		fmt.Printf("%s = %q\n", name, string(data))
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
