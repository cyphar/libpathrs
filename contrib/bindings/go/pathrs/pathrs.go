// libpathrs: safe path resolution on Linux
// Copyright (C) 2019, 2020 Aleksa Sarai <cyphar@cyphar.com>
// Copyright (C) 2020 Maxim Zhiburt <zhiburt@gmail.com>
// Copyright (C) 2019, 2020 SUSE LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// Package pathrs provides bindings for libpathrs, a library for safe path
// resolution on Linux.
package pathrs

// #cgo LDFLAGS: -lpathrs
// #include <pathrs.h>
import "C"

import (
	"crypto/rand"
	"fmt"
	"os"
	"runtime"
	"strings"
	"unsafe"
)

// pathrsObject is implemented by all wrappers of FFI-managed objects.
type pathrsObject interface {
	// inner returns the (type, pointer) tuple for the underlying FFI-managed
	// object.
	inner() (C.pathrs_type_t, unsafe.Pointer)
}

// Ensure that all FFI-managed objects implement pathrsObject at compile-time.
var _ pathrsObject = &Root{}
var _ pathrsObject = &Handle{}

// Root is a handle to the root of a directory tree to resolve within. The only
// purpose of this "root handle" is to perform operations within the directory
// tree, or to get Handles to inodes within the directory tree.
//
// At the time of writing, it is considered a *VERY BAD IDEA* to open a Root
// inside a possibly-attacker-controlled directory tree. While we do have
// protections that should defend against it (for both drivers), it's far more
// dangerous than just opening a directory tree which is not inside a
// potentially-untrusted directory.
type Root struct {
	root *C.pathrs_root_t
}

// inner returns the (type, pointer) tuple for the underlying FFI-managed
// object.
func (r *Root) inner() (C.pathrs_type_t, unsafe.Pointer) {
	return C.PATHRS_ROOT, unsafe.Pointer(r.root)
}

// Open creates a new Root handle to the directory at the given path.
func Open(path string) (*Root, error) {
	runtime.LockOSThread()
	defer runtime.UnlockOSThread()

	rootInner := C.pathrs_open(C.CString(path))
	root := &Root{root: rootInner}
	return root, fetchError(root)
}

// RootFromRaw creates a new Root handle from an exisitng file handle. The
// handle will be copied by this method, so the original handle should still be
// freed by the caller.
//
// This is effectively the inverse operation of Root.IntoRaw, and is used for
// "deserialising" pathrs root handles.
func RootFromRaw(file *os.File) (*Root, error) {
	runtime.LockOSThread()
	defer runtime.UnlockOSThread()

	fd := file.Fd()
	rootInner := (*C.pathrs_root_t)(C.pathrs_from_fd(C.PATHRS_ROOT, C.int(fd)))
	root := &Root{root: rootInner}
	return root, fetchError(root)
}

// Resolve resolves the given path within the Root's directory tree, and return
// a Handle to the resolved path. The path must already exist, otherwise an
// error will occur.
func (r *Root) Resolve(path string) (*Handle, error) {
	runtime.LockOSThread()
	defer runtime.UnlockOSThread()

	handle := C.pathrs_resolve(r.root, C.CString(path))
	return &Handle{handle: handle}, fetchError(r)
}

// Create creates a file within the Root's directory tree at the given path,
// and returns a handle to the file. The provided mode is used for the new file
// (the process's umask applies).
func (r *Root) Create(path string, mode os.FileMode) (*Handle, error) {
	runtime.LockOSThread()
	defer runtime.UnlockOSThread()

	handle := C.pathrs_creat(r.root, C.CString(path), C.uint(mode))
	return &Handle{handle: handle}, fetchError(r)
}

// Rename two paths within a Root's directory tree. The flags argument is
// identical to the RENAME_* flags to the renameat2(2) system call.
func (r *Root) Rename(src, dst string, flags int) error {
	runtime.LockOSThread()
	defer runtime.UnlockOSThread()

	C.pathrs_rename(r.root, C.CString(src), C.CString(dst), C.int(flags))
	return fetchError(r)
}

// Mkdir creates a directory within a Root's directory tree. The provided mode
// is used for the new directory (the process's umask applies).
func (r *Root) Mkdir(path string, mode os.FileMode) error {
	runtime.LockOSThread()
	defer runtime.UnlockOSThread()

	C.pathrs_mkdir(r.root, C.CString(path), C.uint(mode))
	return fetchError(r)
}

// Mknod creates a new device inode of the given type within a Root's directory
// tree. The provided mode is used for the new directory (the process's umask
// applies).
func (r *Root) Mknod(path string, mode os.FileMode, dev int) error {
	runtime.LockOSThread()
	defer runtime.UnlockOSThread()

	C.pathrs_mknod(r.root, C.CString(path), C.uint(mode), C.dev_t(dev))
	return fetchError(r)
}

// Hardlink creates a hardlink within a Root's directory tree. The hardlink is
// created at @path and is a link to @target. Both paths are within the Root's
// directory tree (you cannot hardlink to a different Root or the host).
func (r *Root) Hardlink(path, target string) error {
	runtime.LockOSThread()
	defer runtime.UnlockOSThread()

	C.pathrs_hardlink(r.root, C.CString(path), C.CString(target))
	return fetchError(r)
}

// Symlink creates a symlink within a Root's directory tree. The symlink is
// created at @path and is a link to @target.
func (r *Root) Symlink(path, target string) error {
	runtime.LockOSThread()
	defer runtime.UnlockOSThread()

	C.pathrs_symlink(r.root, C.CString(path), C.CString(target))
	return fetchError(r)
}

// IntoRaw unwraps a file-based libpathrs object to obtain its underlying file
// descriptor.
//
// It is critical that you do not operate on this file descriptor yourself,
// because the security properties of libpathrs depend on users doing all
// relevant filesystem operations through libpathrs.
//
// After this operation, the root should still be freed with root.Close() but
// the root is otherwise invalid and libpathrs will produce an error each time
// it is used.
func (r *Root) IntoRaw() (*os.File, error) {
	runtime.LockOSThread()
	defer runtime.UnlockOSThread()

	fd := int(C.pathrs_into_fd(r.inner()))
	if err := fetchError(r); err != nil {
		return nil, err
	}

	name, err := randName(32)
	if err != nil {
		return nil, err
	}
	return os.NewFile(uintptr(fd), "pathrs-raw-root:"+name), nil
}

// Clone creates a copy of a Root handle, such that it has a separate lifetime
// to the original (while refering to the same underlying directory).
func (r *Root) Clone() (*Root, error) {
	runtime.LockOSThread()
	defer runtime.UnlockOSThread()

	newRoot := (*C.pathrs_root_t)(C.pathrs_duplicate(r.inner()))
	return &Root{root: newRoot}, fetchError(r)
}

// Close frees all of the resources used by the Root handle. The handle must
// not be used for any future operations.
func (r *Root) Close() {
	if r != nil {
		C.pathrs_free(r.inner())
		// Make sure double-frees don't cause segfaults.
		r.root = nil
	}
}

// Handle is a handle for a path within a given Root. This handle references an
// already-resolved path which can be used for only one purpose -- to "re-open"
// the handle and get an actual fs::File which can be used for ordinary
// operations.
//
// It is critical that perform all relevant operations through this Handle
// (rather than fetching the file descriptor yourself with IntoRaw), because
// the security properties of libpathrs depend on users doing all relevant
// filesystem operations through libpathrs.
type Handle struct {
	handle *C.pathrs_handle_t
}

// inner returns the (type, pointer) tuple for the underlying FFI-managed
// object.
func (h *Handle) inner() (C.pathrs_type_t, unsafe.Pointer) {
	return C.PATHRS_HANDLE, unsafe.Pointer(h.handle)
}

// HandleFromRaw creates a new Handle from an exisitng file handle. The handle
// will be copied by this method, so the original handle should still be freed
// by the caller.
//
// This is effectively the inverse operation of Handle.IntoRaw, and is used for
// "deserialising" pathrs root handles.
func HandleFromRaw(file *os.File) (*Handle, error) {
	runtime.LockOSThread()
	defer runtime.UnlockOSThread()

	fd := file.Fd()
	handleInner := (*C.pathrs_handle_t)(C.pathrs_from_fd(C.PATHRS_HANDLE, C.int(fd)))
	handle := &Handle{handle: handleInner}
	return handle, fetchError(handle)
}

// Open creates an "upgraded" file handle to the file referenced by the Handle.
// Note that the original Handle is not consumed by this operation, and can be
// opened multiple times.
//
// The handle returned is only usable for reading, and this is method is
// shorthand for handle.OpenFile(os.O_RDONLY).
func (h *Handle) Open() (*os.File, error) {
	return h.OpenFile(os.O_RDONLY)
}

// OpenFile creates an "upgraded" file handle to the file referenced by the
// Handle. Note that the original Handle is not consumed by this operation, and
// can be opened multiple times.
//
// The provided flags indicate which open(2) flags are used to create the new
// handle.
func (h *Handle) OpenFile(flags int) (*os.File, error) {
	runtime.LockOSThread()
	defer runtime.UnlockOSThread()

	fd := C.pathrs_reopen(h.handle, C.int(flags))
	if err := fetchError(h); err != nil {
		return nil, err
	}

	name, err := randName(32)
	if err != nil {
		return nil, err
	}
	return os.NewFile(uintptr(fd), "pathrs-handle:"+name), nil
}

// IntoRaw unwraps a file-based libpathrs object to obtain its underlying file
// descriptor.
//
// It is critical that you do not operate on this file descriptor yourself,
// because the security properties of libpathrs depend on users doing all
// relevant filesystem operations through libpathrs.
//
// After this operation, the handle should still be freed with handle.Close()
// but the handle is otherwise invalid and libpathrs will produce an error each
// time it is used.
func (h *Handle) IntoRaw() (*os.File, error) {
	runtime.LockOSThread()
	defer runtime.UnlockOSThread()

	fd := int(C.pathrs_into_fd(h.inner()))
	if fd < 0 {
		return nil, fetchError(h)
	}

	name, err := randName(32)
	if err != nil {
		return nil, err
	}
	return os.NewFile(uintptr(fd), "pathrs-raw-handle:"+name), nil
}

// Clone creates a copy of a Handle, such that it has a separate lifetime to
// the original (while refering to the same underlying file).
func (h *Handle) Clone() (*Handle, error) {
	runtime.LockOSThread()
	defer runtime.UnlockOSThread()

	newHandle := (*C.pathrs_handle_t)(C.pathrs_duplicate(h.inner()))
	return &Handle{handle: newHandle}, fetchError(h)
}

// Close frees all of the resources used by the Handle. The handle must not be
// used for any future operations.
func (h *Handle) Close() {
	if h != nil {
		C.pathrs_free(h.inner())
		// Make sure double-frees don't cause segfaults.
		h.handle = nil
	}
}

// randName generates a random hexadecimal name that is used for the Go-level
// "file name" of libpathrs-generated files, and can be used to help with
// debugging.
func randName(k int) (string, error) {
	randBuf := make([]byte, k/2)

	if n, err := rand.Read(randBuf); err != nil {
		return "", err
	} else if n != len(randBuf) {
		return "", fmt.Errorf("rand.Read didn't return enough bytes (%d != %d)", n, len(randBuf))
	}

	var nameBuf strings.Builder
	for _, b := range randBuf {
		nameBuf.WriteString(fmt.Sprintf("%.2x", b))
	}
	return nameBuf.String(), nil
}