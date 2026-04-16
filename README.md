## `libpathrs` ##

[![rust docs](https://img.shields.io/docsrs/pathrs?logo=rust&logoColor=white&color=orange)](https://docs.rs/pathrs/)
[![go docs](https://pkg.go.dev/badge/cyphar.com/go-pathrs.svg)](https://pkg.go.dev/cyphar.com/go-pathrs)
[![PyPI package](https://img.shields.io/pypi/v/pathrs?logo=python&logoColor=white)](https://pypi.org/project/pathrs/)

[![msrv](https://img.shields.io/crates/msrv/pathrs?logo=rust&logoColor=white)](Cargo.toml)
[![dependency status](https://deps.rs/repo/github/cyphar/libpathrs/status.svg)](https://deps.rs/repo/github/cyphar/libpathrs)

[![codecov](https://codecov.io/github/cyphar/libpathrs/graph/badge.svg?token=ITPWXADXPC)](https://codecov.io/github/cyphar/libpathrs)
[![rust-ci build status](https://github.com/cyphar/libpathrs/actions/workflows/rust.yml/badge.svg)](https://github.com/cyphar/libpathrs/actions/workflows/rust.yml)
[![bindings-c build status](https://github.com/cyphar/libpathrs/actions/workflows/bindings-c.yml/badge.svg)](https://github.com/cyphar/libpathrs/actions/workflows/bindings-c.yml)
[![bindings-go build status](https://github.com/cyphar/libpathrs/actions/workflows/bindings-go.yml/badge.svg)](https://github.com/cyphar/libpathrs/actions/workflows/bindings-go.yml)
[![bindings-python build status](https://github.com/cyphar/libpathrs/actions/workflows/bindings-python.yml/badge.svg)](https://github.com/cyphar/libpathrs/actions/workflows/bindings-python.yml)

This library implements a set of C-friendly APIs (written in Rust) to make path
resolution within a potentially-untrusted directory safe on GNU/Linux. [There
are countless examples of security vulnerabilities caused by bad handling of
paths][avoidable-issues]; this library provides an easy-to-use set of VFS APIs
to avoid those kinds of issues.

[avoidable-issues]: ./docs/avoidable-vulnerabilities.md

### Example ###

#### Root and Handle API ####

Here is a toy example of using this library to open a path (`/etc/passwd`)
inside a root filesystem (`/path/to/root`) safely. More detailed examples can
be found in `examples/` and `tests/`.

```c
#include <pathrs.h>

int get_my_fd(void)
{
	const char *root_path = "/path/to/root";
	const char *unsafe_path = "/etc/passwd";

	int liberr = 0;
	int root = -EBADF,
		handle = -EBADF,
		fd = -EBADF;

	root = pathrs_open_root(root_path);
	if (IS_PATHRS_ERR(root)) {
		liberr = root;
		goto err;
	}

	handle = pathrs_inroot_resolve(root, unsafe_path);
	if (IS_PATHRS_ERR(handle)) {
		liberr = handle;
		goto err;
	}

	fd = pathrs_reopen(handle, O_RDONLY);
	if (IS_PATHRS_ERR(fd)) {
		liberr = fd;
		goto err;
	}

err:
	if (IS_PATHRS_ERR(liberr)) {
		pathrs_error_t *error = pathrs_errorinfo(liberr);
		fprintf(stderr, "Uh-oh: %s (errno=%d)\n", error->description, error->saved_errno);
		pathrs_errorinfo_free(error);
	}
	close(root);
	close(handle);
	return fd;
}
```

#### Safe `procfs` API ####

`libpathrs` also provides a set of primitives to safely interact with `procfs`.
This is very important for some programs (such as container runtimes), because
`/proc` has several key system administration purposes that make it different
to other filesystems. It particular, `/proc` is used:

1. As a mechanism for doing certain filesystem operations through
   `/proc/self/fd/...` (and other similar magic-links) that cannot be done by
   other means.
1. As a source of true information about processes and the general system (such
   as by looking `/proc/$pid/status`).
1. As an administrative tool for managing processes (such as setting LSM labels
   like `/proc/self/attr/apparmor/exec`).

These operations have stronger requirements than regular filesystems. For (1)
we need to open the magic-link for real (magic-links are symlinks that are not
resolved lexically, they are in-kernel objects that warp you to other files
without doing a regular path lookup) which much harder to do safely (even with
`openat2`). For (2) and (3) we have the requirement that we need to open a
specific file, not just any file within `/proc` (if there are overmounts or
symlinks) which is not the case `pathrs_inroot_resolve()`. As a result, it is
necessary to take far more care when doing operations of `/proc` and
`libpathrs` provides very useful helper to do this. Failure to do so can lead
to security issues such as those in [CVE-2019-16884][cve-2019-16884] and
[CVE-2019-19921][cve-2019-19921].

In addition, with the [new mount API][lwn-newmount] (`fsopen(2)` and
`open_tree(2)` in particular, added in Linux 5.2), it is possible to get a
totally private `procfs` handle that can be used without worrying about racing
mount operations. `libpathrs` will try to use this if it can (this usually
requires root).

Here are a few examples of practical things you might want to do with
`libpathrs`'s `procfs` API:

```c
/*
 * Safely get an fd to /proc/self/exe. This is something runc does to re-exec
 * itself during the container setup process.
 */
int get_self_exe(void)
{
    /* This follows the trailing magic-link! */
    int fd = pathrs_proc_open(PATHRS_PROC_SELF, "exe", O_PATH);
    if (IS_PATHRS_ERR(fd)) {
        pathrs_error_t *error = pathrs_errorinfo(fd);
        /* ... print the error ... */
        pathrs_errorinfo_free(error);
        return -1;
    }
    return fd;
}

/*
 * Safely set the AppArmor exec label for the current process. This is
 * something runc does while configuring the container process.
 */
int write_apparmor_label(const char *label)
{
    int fd, err;

    /*
     * Note the usage of O_NOFOLLOW here. You should use O_NOFOLLOW except in
     * the very rare case where you need to open a magic-link or you really
     * want to follow a trailing symlink.
     */
    fd = pathrs_proc_open(PATHRS_PROC_SELF, "attr/apparmor/exec",
                          O_WRONLY|O_NOFOLLOW);
    if (IS_PATHRS_ERR(fd)) {
        pathrs_error_t *error = pathrs_errorinfo(fd);
        /* ... print the error ... */
        pathrs_errorinfo_free(error);
        return -1;
    }

    err = write(fd, label, strlen(label));
    close(fd);
    return err;
}

/*
 * Sometimes you need to get the "real" path of a file descriptor. This path
 * MUST NOT be used for actual filesystem operations, because it's possible for
 * an attacker to move the file or change one of the path components to a
 * symlink, which could lead to you operating on files you didn't expect
 * (including host files if you're a container runtime).
 *
 * In most cases, this kind of function would be used for diagnostic purposes
 * (such as in error messages, to provide context about what file the error is
 * in relation to).
 */
char *get_unsafe_path(int fd)
{
    char *fdpath;

    if (asprintf(&fdpath, "fd/%d", fd) < 0)
        return NULL;

    int linkbuf_size = 128;
    char *linkbuf = malloc(size);
    if (!linkbuf)
        goto err;
    for (;;) {
        int len = pathrs_proc_readlink(PATHRS_PROC_THREAD_SELF,
                                       fdpath, linkbuf, linkbuf_size);
        if (IS_PATHRS_ERR(len)) {
            pathrs_error_t *error = pathrs_errorinfo(fd);
            /* ... print the error ... */
            pathrs_errorinfo_free(error);
            goto err;
        }

        if (len <= linkbuf_size)
            break;

        linkbuf_size = len;
        linkbuf = realloc(linkbuf, linkbuf_size);
        if (!linkbuf)
            goto err;
    }

    free(fdpath);
    return linkbuf;

err:
    free(fdpath);
    free(linkbuf);
    return NULL;
}
```

[cve-2019-16884]: https://nvd.nist.gov/vuln/detail/CVE-2019-16884
[cve-2019-19921]: https://nvd.nist.gov/vuln/detail/CVE-2019-19921
[lwn-newmount]: https://lwn.net/Articles/759499/

### Kernel Support ###

At the moment, `libpathrs` only works on Linux as it was designed around
Linux-only APIs that are necessary to provide safe path operations. In future,
we plan to expand support for other Unix-like operating systems.

While `libpathrs` will function on very old kernels (in theory back to Linux
2.6.39, though we do not currently test this) we *strongly* recommend using at
least Linux 5.6 to get a reasonable amount of protection against various
attacks. The oldest Linux kernel which currently supports [all of the features
we use for hardening][kernel-feature-list] is Linux 6.8.

[kernel-feature-list]: ./docs/kernel-features.md

### License ###

`SPDX-License-Identifier: MPL-2.0 OR LGPL-3.0-or-later`

`libpathrs` is licensed under the terms of the [Mozilla Public License version
2.0][MPL-2.0] or the [GNU Lesser General Public License version 3][LGPL-3.0],
at your option.

Unless otherwise stated, by intentionally submitting any Contribution (as
defined by the Mozilla Public License version 2.0) for inclusion into the
`libpathrs` project, you are agreeing to dual-license your Contribution as
above, without any additional terms or conditions.

```
libpathrs: safe path resolution on Linux
Copyright (C) 2019-2025 SUSE LLC
Copyright (C) 2026 Aleksa Sarai <cyphar@cyphar.com>

== MPL-2.0 ==

 This Source Code Form is subject to the terms of the Mozilla Public
 License, v. 2.0. If a copy of the MPL was not distributed with this
 file, You can obtain one at https://mozilla.org/MPL/2.0/.

Alternatively, this Source Code Form may also (at your option) be used
under the terms of the GNU Lesser General Public License Version 3, as
described below:

== LGPL-3.0-or-later ==

 This program is free software: you can redistribute it and/or modify it
 under the terms of the GNU Lesser General Public License as published by
 the Free Software Foundation, either version 3 of the License, or (at
 your option) any later version.

 This program is distributed in the hope that it will be useful, but
 WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY
 or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License
 for more details.

 You should have received a copy of the GNU Lesser General Public License
 along with this program. If not, see <https://www.gnu.org/licenses/>.
```

[MPL-2.0]: LICENSE.MPL-2.0
[LGPL-3.0]: LICENSE.LGPL-3.0

#### Bindings ####

`SPDX-License-Identifier: MPL-2.0`

The language-specific bindings (the code in `contrib/bindings/` and
`go-pathrs/`) are licensed under the Mozilla Public License version 2.0
(available in [`LICENSE.MPL-2.0`][MPL-2.0]).

**NOTE**: If you compile `libpathrs.so` into your binary statically, you still
need to abide by the license terms of the main `libpathrs` project.

```
libpathrs: safe path resolution on Linux
Copyright (C) 2019-2025 SUSE LLC
Copyright (C) 2026 Aleksa Sarai <cyphar@cyphar.com>

This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
```

[MPL-2.0]: LICENSE.MPL-2.0

#### Examples ####

`SPDX-License-Identifier: MPL-2.0`

The example code in `examples/` is licensed under the Mozilla Public License
version 2.0 (available in [`LICENSE.MPL-2.0`][MPL-2.0]).

```
libpathrs: safe path resolution on Linux
Copyright (C) 2019-2025 SUSE LLC
Copyright (C) 2026 Aleksa Sarai <cyphar@cyphar.com>

This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
```
