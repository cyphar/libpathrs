// SPDX-License-Identifier: MPL-2.0 OR LGPL-3.0-or-later
/*
 * libpathrs: safe path resolution on Linux
 * Copyright (C) 2019-2025 SUSE LLC
 * Copyright (C) 2026 Aleksa Sarai <cyphar@cyphar.com>
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
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_CAPI");
    println!("cargo:rerun-if-env-changed=LIBPATHRS_CAPI_BUILDMODE");
    if cfg!(feature = "capi") {
        // Add DT_SONAME and other ELF metadata to our cdylibs. We can't check
        // the crate-type here directly, so we have our Makefile define a
        // special environment variable instead (for whatever reason, even
        // --cfg=... doesn't seem to propagate to build.rs. This means that each
        // crate-type needs to be built separately (with --cargo-type).
        //
        // The alternative would be to make cdylib/symvers a feature, which
        // seems even more prone to breakage -- at least your build will
        // explicitly fail if you don't specifying LIBPATHRS_CAPI_BUILDMODE.
        let is_cdylib = match env::var("LIBPATHRS_CAPI_BUILDMODE")
            .unwrap_or_else(|_| "".to_string())
            .as_str()
        {
            "cdylib" => {
                println!("cargo:rustc-cfg=cdylib");
                true
            }
            "staticlib" | "" => false,
            v => panic!("unknown LIBPATHRS_CAPI_BUILDMODE={v} value"),
        };
        if is_cdylib {
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
            LIBPATHRS_0.1 {{ }};
            LIBPATHRS_0.2 {{ local: *; }} LIBPATHRS_0.1;
            "#
            )
            .expect("write version script");
            println!("cargo:rustc-cdylib-link-arg=-Wl,--version-script={version_script_path}");

            // The above version script (and our .symver setup) conflicts with the
            // version script and options used by Rust when linking with GNU ld.
            // Thankfully, lld Just Works(TM) out of the box so we can use it.
            //
            // Rust 1.90 switched to lld by default for x86, but for older versions
            // and other architectures it is necessary to specify the linker as lld
            // (there was also a rustflag for this but it was unstable until Rust
            // 1.90).
            //
            // Unfortunately, while there are some clever tricks you could use for
            // GNU ld (such as writing an ld wrapper and executing it with "cc -B"),
            // doing so produces useless symbol versions so it's better to just
            // require lld. Debian bullseye and later all have lld, so this is a
            // non-issue for packagers.
            println!("cargo:rustc-cdylib-link-arg=-fuse-ld=lld");
        }
    }
}
