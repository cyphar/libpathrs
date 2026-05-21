// SPDX-License-Identifier: MPL-2.0
/*
 * libpathrs: safe path resolution on Linux
 * Copyright (C) 2026 Aleksa Sarai <cyphar@cyphar.com>
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use anyhow::{anyhow, Error};
use clap::{ArgMatches, Command};

pub(crate) fn cli() -> Command {
    Command::new("version").about("version information")
}

pub(crate) fn subcommand(matches: &ArgMatches) -> Result<(), Error> {
    if let Some((subcommand, _)) = matches.subcommand() {
        // We should never end up here.
        Err(anyhow!("unknown 'version' subcommand '{subcommand}'"))?;
    }

    println!("VERSION {}", pathrs::VERSION);
    Ok(())
}
