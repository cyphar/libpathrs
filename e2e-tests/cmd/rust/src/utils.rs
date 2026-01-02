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

use pathrs::{
    flags::OpenFlags,
    procfs::{ProcfsBase, ProcfsHandle},
    Handle,
};

use std::{
    ffi::OsStr,
    fs::{File, Permissions},
    os::unix::{
        fs::PermissionsExt,
        io::{AsFd, AsRawFd},
    },
    path::PathBuf,
};

use anyhow::Error;
use clap::{builder::TypedValueParser, error::ErrorKind as ClapErrorKind, Arg, ArgAction, Command};

pub(crate) fn subpath_arg(name: impl Into<clap::Id>) -> Arg {
    Arg::new(name).value_parser(clap::value_parser!(PathBuf))
}

pub(crate) fn toggle_arg(name: &str, help: &str) -> Vec<Arg> {
    let name = name.to_string();
    let no_name = format!("no-{name}");

    // In order for the default to be "--{name}" we need to define the "{name}"
    // id with ArgAction::SetFalse which leads to the annoying situation where
    // the id for the "--{name}" flag cannot be "name" so we need a dummy name
    // to use.
    let yes_name = format!("DUMMY-yes-{name}");
    vec![
        Arg::new(&no_name)
            .id(&name)
            .long(&no_name)
            .help(format!("disable {help}"))
            .action(ArgAction::SetFalse),
        Arg::new(&yes_name)
            .long(&name)
            .help(format!("{help} [default]"))
            .action(ArgAction::SetTrue)
            .overrides_with(&name),
    ]
}

#[derive(Debug, Clone, Copy)]
struct OFlagParser;

impl TypedValueParser for OFlagParser {
    type Value = OpenFlags;

    fn parse_ref(
        &self,
        _cmd: &Command,
        _arg: Option<&Arg>,
        value: &OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let value = value
            .to_str()
            .ok_or_else(|| {
                clap::Error::raw(
                    ClapErrorKind::InvalidUtf8,
                    "oflags contained invalid utf8 characters",
                )
            })?
            .to_uppercase();

        let mut parsed = OpenFlags::empty();
        for flagname in value.split([',', '|']) {
            let flagname = if flagname.starts_with("O_") {
                flagname.to_owned()
            } else {
                format!("O_{flagname}")
            };
            parsed |= OpenFlags::from_name(&flagname).ok_or_else(|| {
                clap::Error::raw(
                    ClapErrorKind::ValueValidation,
                    format!("{flagname} is not a valid OpenFlag value"),
                )
            })?;
        }

        Ok(parsed)
    }
}

pub(crate) fn oflags_arg(name: impl Into<clap::Id>, help: &str) -> Arg {
    Arg::new(name)
        .help(format!("{help} (comma- or |-separated)"))
        .value_name("O_*")
        .value_parser(OFlagParser)
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ModeParser;

impl TypedValueParser for ModeParser {
    type Value = Permissions;

    fn parse_ref(
        &self,
        _cmd: &Command,
        _arg: Option<&Arg>,
        value: &OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let value = value.to_str().ok_or_else(|| {
            clap::Error::raw(
                ClapErrorKind::InvalidUtf8,
                "mode contained invalid utf8 characters",
            )
        })?;

        u32::from_str_radix(value.strip_prefix("0o").unwrap_or(value), 8)
            .map(Permissions::from_mode)
            .map_err(|err| {
                clap::Error::raw(
                    ClapErrorKind::ValueValidation,
                    format!("{value} is an invalid octal mode: {err:?}"),
                )
            })
    }
}

fn fd_path<Fd: AsFd>(fd: Fd) -> Result<PathBuf, Error> {
    let fd = fd.as_fd();
    ProcfsHandle::new()?
        .readlink(ProcfsBase::ProcThreadSelf, format!("fd/{}", fd.as_raw_fd()))
        .map_err(Into::into)
}

pub(crate) fn print_handle(h: &Handle) -> Result<(), Error> {
    println!("HANDLE-PATH {}", fd_path(h)?.to_string_lossy());
    Ok(())
}

pub(crate) fn print_file(f: &File) -> Result<(), Error> {
    println!("FILE-PATH {}", fd_path(f)?.to_string_lossy());
    // TODO: Do some other operations on files.
    Ok(())
}
