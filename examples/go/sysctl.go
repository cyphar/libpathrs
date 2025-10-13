// SPDX-License-Identifier: LGPL-3.0-or-later
/*
 * libpathrs: safe path resolution on Linux
 * Copyright (C) 2019-2025 Aleksa Sarai <cyphar@cyphar.com>
 * Copyright (C) 2019-2025 SUSE LLC
 *
 * This program is free software: you can redistribute it and/or modify it under
 * the terms of the GNU Lesser General Public License as published by the Free
 * Software Foundation, either version 3 of the License, or (at your option) any
 * later version.
 *
 * This program is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A
 * PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU Lesser General Public License along
 * with this program. If not, see <https://www.gnu.org/licenses/>.
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
