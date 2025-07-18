/*
 * libpathrs: safe path resolution on Linux
 * Copyright (C) 2019-2021 Aleksa Sarai <cyphar@cyphar.com>
 * Copyright (C) 2019-2021 SUSE LLC
 *
 * This program is free software: you can redistribute it and/or modify it
 * under the terms of the GNU Lesser General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or (at your
 * option) any later version.
 *
 * This program is distributed in the hope that it will be useful, but
 * WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY
 * or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License
 * for more details.
 *
 * You should have received a copy of the GNU Lesser General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

use std::env;

fn main() {
    // Add DT_SONAME to our cdylibs. We can't check the crate-type here
    // directly, but we can at least avoid needless warnings for "cargo build"
    // by only emitting this when the capi feature is enabled.
    if cfg!(feature = "capi") {
        let name = "pathrs";
        let major = env::var("CARGO_PKG_VERSION_MAJOR").unwrap();
        println!("cargo:rustc-cdylib-link-arg=-Wl,-soname,lib{name}.so.{major}");
    }
}
