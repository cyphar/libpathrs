//go:build linux

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

package pathrs

import (
	"fmt"
	"os"

	"golang.org/x/sys/unix"

	"cyphar.com/go-pathrs/procfs"
)

// dupFd makes a duplicate of the given fd.
func dupFd(fd uintptr, name string) (*os.File, error) {
	newFd, err := unix.FcntlInt(fd, unix.F_DUPFD_CLOEXEC, 0)
	if err != nil {
		return nil, fmt.Errorf("fcntl(F_DUPFD_CLOEXEC): %w", err)
	}
	return os.NewFile(uintptr(newFd), name), nil
}

//nolint:cyclop // this function needs to handle a lot of cases
func toUnixMode(mode os.FileMode) (uint32, error) {
	sysMode := uint32(mode.Perm())
	switch mode & os.ModeType { //nolint:exhaustive // we only care about ModeType bits
	case 0:
		sysMode |= unix.S_IFREG
	case os.ModeDir:
		sysMode |= unix.S_IFDIR
	case os.ModeSymlink:
		sysMode |= unix.S_IFLNK
	case os.ModeCharDevice | os.ModeDevice:
		sysMode |= unix.S_IFCHR
	case os.ModeDevice:
		sysMode |= unix.S_IFBLK
	case os.ModeNamedPipe:
		sysMode |= unix.S_IFIFO
	case os.ModeSocket:
		sysMode |= unix.S_IFSOCK
	default:
		return 0, fmt.Errorf("invalid mode filetype %+o", mode)
	}
	if mode&os.ModeSetuid != 0 {
		sysMode |= unix.S_ISUID
	}
	if mode&os.ModeSetgid != 0 {
		sysMode |= unix.S_ISGID
	}
	if mode&os.ModeSticky != 0 {
		sysMode |= unix.S_ISVTX
	}
	return sysMode, nil
}

// withFileFd is a more ergonomic wrapper around file.SyscallConn().Control().
func withFileFd[T any](file *os.File, fn func(fd uintptr) (T, error)) (T, error) {
	conn, err := file.SyscallConn()
	if err != nil {
		return *new(T), err
	}
	var (
		ret      T
		innerErr error
	)
	if err := conn.Control(func(fd uintptr) {
		ret, innerErr = fn(fd)
	}); err != nil {
		return *new(T), err
	}
	return ret, innerErr
}

// dupFile makes a duplicate of the given file.
func dupFile(file *os.File) (*os.File, error) {
	return withFileFd(file, func(fd uintptr) (*os.File, error) {
		return dupFd(fd, file.Name())
	})
}

// mkFile creates a new *os.File from the provided file descriptor. However,
// unlike os.NewFile, the file's Name is based on the real path (provided by
// /proc/self/fd/$n).
func mkFile(fd uintptr) (*os.File, error) {
	proc, err := procfs.Open()
	if err != nil {
		return nil, err
	}
	defer proc.Close() //nolint:errcheck // Close errors are not critical

	fdPath := fmt.Sprintf("fd/%d", fd)
	fdName, err := proc.Readlink(procfs.ProcThreadSelf, fdPath)
	if err != nil {
		_ = unix.Close(int(fd))
		return nil, fmt.Errorf("failed to fetch real name of fd %d: %w", fd, err)
	}
	// TODO: Maybe we should prefix this name with something to indicate to
	// users that they must not use this path as a "safe" path. Something like
	// "//pathrs-handle:/foo/bar"?
	return os.NewFile(fd, fdName), nil
}
