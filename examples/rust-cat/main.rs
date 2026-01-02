// SPDX-License-Identifier: MPL-2.0
/*
 * libpathrs: safe path resolution on Linux
 * Copyright (C) 2019-2025 SUSE LLC
 * Copyright (C) 2026 Aleksa Sarai <cyphar@cyphar.com>
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

/*
 * File: examples/rust-cat/main.rs
 *
 * An example program which opens a file inside a root and outputs its
 * contents using libpathrs.
 */

use pathrs::{flags::OpenFlags, Root};

use std::io::{prelude::*, BufReader};

use anyhow::{Context, Error};
use clap::{Arg, Command};

fn main() -> Result<(), Error> {
    let m = Command::new("cat")
        // MSRV(1.67): Use clap::crate_authors!.
        .author("Aleksa Sarai <cyphar@cyphar.com>")
        .version(clap::crate_version!())
        .arg(Arg::new("root").value_name("ROOT"))
        .arg(Arg::new("unsafe-path").value_name("PATH"))
        .about("")
        .get_matches();

    let root_path = m
        .get_one::<String>("root")
        .context("required root argument not provided")?;
    let unsafe_path = m
        .get_one::<String>("unsafe-path")
        .context("required unsafe-path argument not provided")?;

    let root = Root::open(root_path).context("open root failed")?;
    let file = root
        .open_subpath(unsafe_path, OpenFlags::O_RDONLY)
        .context("open unsafe path in root")?;

    let reader = BufReader::new(file);
    for line in reader.lines() {
        println!("{}", line.context("read lines")?);
    }
    Ok(())
}
