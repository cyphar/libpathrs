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

use crate::utils;
use pathrs::{
    flags::OpenFlags,
    procfs::{ProcfsBase, ProcfsHandleBuilder, ProcfsHandleRef},
};

use std::{ffi::OsStr, path::PathBuf};

use anyhow::{anyhow, Error};
use clap::{
    builder::TypedValueParser, error::ErrorKind as ClapErrorKind, Arg, ArgAction, ArgMatches,
    Command,
};

#[derive(Debug, Clone, Copy)]
struct ProcfsBaseParser;

impl TypedValueParser for ProcfsBaseParser {
    type Value = ProcfsBase;

    fn parse_ref(
        &self,
        _cmd: &Command,
        _arg: Option<&Arg>,
        value: &OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let value = value.to_str().ok_or_else(|| {
            clap::Error::raw(
                ClapErrorKind::InvalidUtf8,
                "procfs base contained invalid utf8 characters",
            )
        })?;

        match (value, value.strip_prefix("pid=")) {
            (_, Some(pid)) => Ok(ProcfsBase::ProcPid(pid.parse().map_err(|err| {
                clap::Error::raw(
                    ClapErrorKind::ValueValidation,
                    format!("{value} is an invalid octal mode: {err:?}"),
                )
            })?)),
            ("root", _) => Ok(ProcfsBase::ProcRoot),
            ("self", _) => Ok(ProcfsBase::ProcSelf),
            ("thread-self", _) => Ok(ProcfsBase::ProcThreadSelf),
            (value, None) => Err(clap::Error::raw(
                ClapErrorKind::ValueValidation,
                format!("{value} is an invalid procfs base"),
            )),
        }
    }
}

fn open_cli() -> Command {
    Command::new("open")
        .about("open a path in procfs")
        .arg(
            utils::oflags_arg("oflags", "O_* flags to use when opening the file")
                .long("oflags")
                .default_value("O_RDONLY"),
        )
        .args(utils::toggle_arg("follow", "follow trailing symlinks"))
        .arg(
            utils::subpath_arg("subpath")
                .help("path inside procfs base")
                .required(true),
        )
}

fn open(procfs: ProcfsHandleRef<'_>, base: ProcfsBase, matches: &ArgMatches) -> Result<(), Error> {
    let subpath = matches
        .get_one::<PathBuf>("subpath")
        .expect("subpath should always be set");
    let follow = *matches
        .get_one::<bool>("follow")
        .expect("follow should be set");
    let oflags = *matches
        .get_one::<OpenFlags>("oflags")
        .expect("oflags should always be set");

    let f = if follow {
        procfs.open_follow(base, subpath, oflags)
    } else {
        procfs.open(base, subpath, oflags)
    }?;
    utils::print_file(&f)?;

    Ok(())
}

fn readlink_cli() -> Command {
    Command::new("readlink")
        .about("read the target path of a symbolic link in procfs")
        .arg(
            utils::subpath_arg("subpath")
                .help("path inside procfs base")
                .required(true),
        )
}

fn readlink(
    procfs: ProcfsHandleRef<'_>,
    base: ProcfsBase,
    matches: &ArgMatches,
) -> Result<(), Error> {
    let subpath = matches
        .get_one::<PathBuf>("subpath")
        .expect("subpath should always be set");

    let link_target = procfs.readlink(base, subpath)?;
    println!("LINK-TARGET {}", link_target.to_string_lossy());
    Ok(())
}

pub(crate) fn cli() -> Command {
    Command::new("procfs")
        .about("ProcfsHandle::* operations")
        .arg(
            Arg::new("unmasked")
                .long("unmasked")
                .help("use unmasked procfs handle")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("base")
                .long("base")
                .help("base path for procfs operations (root, pid=<n>, self, thread-self)")
                .value_name("PROC_*")
                .default_value("root")
                .value_parser(ProcfsBaseParser)
                .action(ArgAction::Set),
        )
        .subcommand(open_cli())
        .subcommand(readlink_cli())
}

pub(crate) fn subcommand(matches: &ArgMatches) -> Result<(), Error> {
    let procfs = {
        let mut b = ProcfsHandleBuilder::new();

        // MSRV(1.85): Use let chain here (Rust 2024).
        if matches.get_one::<bool>("unmasked") == Some(&true) {
            b.set_unmasked();
        }

        b.build()
    }?;
    let base = *matches
        .get_one::<ProcfsBase>("base")
        .expect("base should always be set");

    match matches.subcommand() {
        Some(("open", sub_matches)) => open(procfs, base, sub_matches),
        Some(("readlink", sub_matches)) => readlink(procfs, base, sub_matches),
        Some((subcommand, _)) => {
            // We should never end up here.
            Err(anyhow!("unknown 'procfs' subcommand '{subcommand}'"))
        }
        None => Err(anyhow!("no 'procfs' subcommand specified")),
    }
}
