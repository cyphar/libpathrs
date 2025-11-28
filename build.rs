// SPDX-License-Identifier: MPL-2.0 OR LGPL-3.0-or-later
/*
 * libpathrs: safe path resolution on Linux
 * Copyright (C) 2019-2025 Aleksa Sarai <cyphar@cyphar.com>
 * Copyright (C) 2019-2025 SUSE LLC
 *
 * == MPL-2.0 ==
 *
 *  This Source Code Form is subject to the terms of the Mozilla Public
 *  License, v. 2.0. If a copy of the MPL was not distributed with this
 *  file, You can obtain one at https://mozilla.org/MPL/2.0/.
 *
 * Alternatively, this Source Code Form may also (at your option) be used
 * under the terms of the GNU Lesser General Public License Version 3, as
 * described below:
 *
 * == LGPL-3.0-or-later ==
 *
 *  This program is free software: you can redistribute it and/or modify it
 *  under the terms of the GNU Lesser General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or (at
 *  your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful, but
 *  WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY  or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General
 * Public License  for more details.
 *
 *  You should have received a copy of the GNU Lesser General Public License
 *  along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

use std::{env, io::Write};

use tempfile::NamedTempFile;

fn main() {
    // Add DT_SONAME and other ELF metadata to our cdylibs. We can't check the
    // crate-type here directly, but we can at least avoid needless warnings for
    // "cargo build" by only emitting this when the capi feature is enabled.
    if cfg!(feature = "capi") {
        let name = "pathrs";
        // TODO: Since we use symbol versioning, it seems quite unlikely that we
        // would ever bump the major version in the SONAME, so we should
        // probably hard-code this or define it elsewhere.
        let major = env::var("CARGO_PKG_VERSION_MAJOR").unwrap();
        println!("cargo:rustc-cdylib-link-arg=-Wl,-soname,lib{name}.so.{major}");

        let (mut version_script_file, version_script_path) =
            NamedTempFile::with_prefix("libpathrs-version-script.")
                .expect("mktemp")
                .keep()
                .expect("persist mktemp");
        let version_script_path = version_script_path
            .to_str()
            .expect("mktemp should be utf-8 safe string");
        writeln!(
            version_script_file,
            // All of the symbol versions are done with in-line .symver entries.
            // This version script is only needed to define the version nodes
            // (and their dependencies).
            // FIXME: "local" doesn't appear to actually hide symbols in the
            // output .so. For more information about getting all of this to
            // work nicely, see <https://internals.rust-lang.org/t/23626>.
            r#"
            LIBPATHRS_0.2 {{ local: *; }} LIBPATHRS_0.1;
            LIBPATHRS_0.1 {{  }};
            "#
        )
        .expect("write version script");
        println!("cargo:rustc-cdylib-link-arg=-Wl,--version-script={version_script_path}");
    }
}
