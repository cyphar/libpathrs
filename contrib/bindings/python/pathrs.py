#!/usr/bin/python3
# libpathrs: safe path resolution on Linux
# Copyright (C) 2019 Aleksa Sarai <cyphar@cyphar.com>
# Copyright (C) 2019 SUSE LLC
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

import re
import os
import sys
import cffi

__all__ = ["Root", "Handle"]

# Resolvers supported by pathrs -- can be configured with PATHRS_RESOLVER.
KERNEL_RESOLVER = "kernel"
EMULATED_RESOLVER = "emulated"

def cstr(pystr):
	return ffi.new("char[]", pystr.encode("utf8"))

def pystr(cstr):
	return ffi.string(cstr).decode("utf8")

def objtype(obj):
	if isinstance(obj, Root):
		return libpathrs_so.PATHRS_ROOT
	elif isinstance(obj, Handle):
		return libpathrs_so.PATHRS_HANDLE
	else:
		raise PathrsError("internal error: %r is not a pathrs object" % (obj,))

def error(obj):
	err = ffi.new("pathrs_error_t *")
	ret = libpathrs_so.pathrs_error(objtype(obj), obj.inner, err)
	if ret <= 0:
		raise PathrsError("internal error", errno=-ret)
	return PathrsError(pystr(err.description), errno=err.errno or None)


class PathrsError(Exception):
	def __init__(self, message, errno=None):
		# Construct Exception.
		super().__init__(message)
		# Basic arguments.
		self.message = message
		self.errno = errno

	def __str__(self):
		if self.errno is None:
			return self.message
		else:
			return "%s (errno=%d)" % (self.message, self.errno)

	def __repr__(self):
		return "PathrsError(%r, errno=%r)" % (self.message, self.errno)


class Handle(object):
	def __init__(self, handle):
		self.inner = handle

	def __del__(self):
		libpathrs_so.pathrs_free(objtype(self), self.inner)

	def reopen(self, flags):
		fd = libpathrs_so.pathrs_reopen(self.inner, flags)
		if fd < 0:
			raise error(self)
		try:
			return os.fdopen(fd)
		except Exception as e:
			os.close(fd)
			raise


class Root(object):
	def __init__(self, path, resolver=None):
		self.inner = None
		path = cstr(path)
		root = libpathrs_so.pathrs_open(path)
		if root == ffi.NULL:
			raise PathrsError("pathrs_root allocation failed")
		# Check the environment for any default resolver override.
		if resolver is None:
			resolver = os.environ.get("PATHRS_RESOLVER")
		# Switch resolvers if requested.
		if resolver is not None:
			resolver = {
				KERNEL_RESOLVER: libpathrs_so.PATHRS_KERNEL_RESOLVER,
				EMULATED_RESOLVER: libpathrs_so.PATHRS_EMULATED_RESOLVER,
			}[resolver]
			libpathrs_so.pathrs_set_resolver(root, resolver)
		self.inner = root

	def __del__(self):
		if self.inner is not None:
			libpathrs_so.pathrs_free(objtype(self), self.inner)

	def resolve(self, path):
		path = cstr(path)
		handle = libpathrs_so.pathrs_inroot_resolve(self.inner, path)
		if handle == ffi.NULL:
			raise error(self)
		return Handle(handle)

	def rename(self, src, dst, flags=0):
		src = cstr(src)
		dst = cstr(dst)
		err = libpathrs_so.pathrs_inroot_rename(self.inner, src, dst, flags)
		if err < 0:
			raise error(self)

	def creat(self, path, mode):
		path = cstr(path)
		handle = libpathrs_so.pathrs_inroot_creat(self.inner, path, mode)
		if handle == ffi.NULL:
			raise error(self)
		return Handle(handle)

	def mkdir(self, path, mode):
		path = cstr(path)
		err = libpathrs_so.pathrs_inroot_mkdir(self.inner, path, mode)
		if err < 0:
			raise error(self)

	def mknod(self, path, mode, dev):
		path = cstr(path)
		err = libpathrs_so.pathrs_inroot_mknod(self.inner, path, mode, dev)
		if err < 0:
			raise error(self)

	def hardlink(self, path, target):
		path = cstr(path)
		target = cstr(target)
		err = libpathrs_so.pathrs_inroot_hardlink(self.inner, path, target)
		if err < 0:
			raise error(self)

	def symlink(self, path, target):
		path = cstr(path)
		target = cstr(target)
		err = libpathrs_so.pathrs_inroot_symlink(self.inner, path, target)
		if err < 0:
			raise error(self)


def __do_load():
	global ffi, libpathrs_so
	ffi = cffi.FFI()

	# TODO: All of this searching code should really be disabled for
	#       "production" users because a privileged binary executed in an
	#       unsafe working directory could be easily tricked into loading (and
	#       thus running) arbitrary code.

	# Figure out where the libpathrs source dir is.
	ROOT_DIR = os.path.dirname(sys.path[0]) or ".."

	# Figure out the header path.
	include_path = "/usr/include/pathrs.h"
	if not os.path.exists(include_path):
		include_path = os.path.join(ROOT_DIR, "include/pathrs.h")
	if not os.path.exists(include_path):
		raise PathrsError("Cannot find 'libpathrs' header.")

	with open(include_path) as f:
		# Strip out #-lines.
		hdr = re.sub(r"^#.*", "", f.read(), flags=re.MULTILINE)
		ffi.cdef("typedef uint32_t dev_t;")
		ffi.cdef(hdr)

	try:
		# First, try system search paths.
		libpathrs_so = ffi.dlopen("pathrs")
	except OSError:
		# Okay, let's try falling back to source.
		paths = []
		for mode in ["debug", "release"]:
			so_path = os.path.join(ROOT_DIR, "target/%s/libpathrs.so" % (mode,))
			if os.path.exists(so_path):
				paths.append(so_path)
		if not paths:
			raise PathrsError("Cannot find 'libpathrs' library.")

		# Use the last-modified library, since that's presumably what we want.
		paths = sorted(paths, key=lambda path: -os.path.getmtime(path))
		libpathrs_so = ffi.dlopen(paths[0])

# Initialise our libpathrs handles.
__do_load()
