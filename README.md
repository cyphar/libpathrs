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

### Examples ###

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

On Linux, libpathrs also provides an API for safe `procfs` operations with
strict path safety [in the `procfs` module][docs.rs-procfs]. [Click
here][procfs-api] to see some concrete examples of its usage.

[docs.rs-procfs]: https://docs.rs/pathrs/latest/pathrs/procfs/index.html
[procfs-api]: ./docs/procfs-api.md

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
