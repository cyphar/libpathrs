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

use pathrs::error::{Error as PathrsError, ErrorKind as PathrsErrorKind};

use std::{env, process::ExitCode};

use anyhow::{anyhow, Context, Error};
use clap::Command;
use errno::Errno;
use rustix::process::{self as rustix_process, DumpableBehavior};

mod procfs;
mod root;
mod utils;
mod version;

fn cli() -> Command {
    Command::new("pathrs-cmd")
        .author("Aleksa Sarai <cyphar@cyphar.com>")
        .subcommand(version::cli())
        .subcommand(root::cli())
        .subcommand(procfs::cli())
}

#[test]
fn verify_app() {
    cli().debug_assert();
}

fn handle_error(func: impl FnOnce() -> Result<(), Error>) -> ExitCode {
    if let Err(err) = func() {
        let mut desc = err.to_string();
        for cause in err.chain() {
            if let Some(err) = cause.downcast_ref::<PathrsError>() {
                // This is basically ErrorKind::errno (which isn't exported
                // currently). This is necessary in order to emulate the
                // behaviour of the binding test programs, as they see the
                // converted errnos as well as the legitimate OsError ones.
                let errno = match err.kind() {
                    PathrsErrorKind::NotImplemented => Some(libc::ENOSYS),
                    PathrsErrorKind::InvalidArgument => Some(libc::EINVAL),
                    PathrsErrorKind::OsError(errno) => errno,
                    PathrsErrorKind::SafetyViolation => Some(libc::EXDEV),
                    _ => None,
                };
                if let Some(errno) = errno {
                    println!("ERRNO {errno} ({})", Errno(errno));
                }
            }
            // Emulate capi's error formatting.
            desc.push_str(": ");
            desc.push_str(&cause.to_string());
        }
        println!("error: {desc}");
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn main() -> ExitCode {
    handle_error(|| {
        if let Ok(dumpable) = env::var("PR_SET_DUMPABLE") {
            match dumpable.as_str() {
                "" => (), // noop
                s => {
                    let dumpable: DumpableBehavior = s
                        .parse::<i32>()
                        .context("PR_SET_DUMPABLE environment variable must be int or ''")?
                        .try_into()
                        .context("invalid PR_SET_DUMPABLE environment variable value")?;

                    rustix_process::set_dumpable_behavior(dumpable)
                        .with_context(|| format!("failed to set PR_SET_DUMPABLE={dumpable:?}"))?;
                }
            }
        }

        let mut args = env::args_os().collect::<Vec<_>>();
        if env::var_os("PATHRS_CMD_UNESCAPE").is_some_and(|v| !v.is_empty()) {
            // We must skip argv[0].
            for arg in args.iter_mut().skip(1) {
                *arg = utils::unescape_arg(arg)
                    .context("unescape argument (PATHRS_CMD_UNESCAPE is set)")?;
            }
        }

        let mut app = cli();

        let matches = app
            .try_get_matches_from_mut(args)
            .unwrap_or_else(|err| err.exit());

        match matches.subcommand() {
            Some(("version", sub_matches)) => version::subcommand(sub_matches),
            Some(("root", sub_matches)) => root::subcommand(sub_matches),
            Some(("procfs", sub_matches)) => procfs::subcommand(sub_matches),
            Some((subcommand, _)) => {
                // We should never end up here.
                app.print_help()?;
                Err(anyhow!("unknown subcommand '{}'", subcommand))
            }
            None => {
                app.print_help()?;
                Err(anyhow!("no subcommand specified"))
            }
        }
    })
}
