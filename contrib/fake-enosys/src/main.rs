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

mod bpf;

use std::{os::unix::process::CommandExt, process::Command as StdCmd};

use anyhow::{anyhow, Context, Error};
use clap::{Arg, ArgAction, Command};
use rustix::{process as rustix_process, thread as rustix_thread};
use syscalls::Sysno;

fn cli() -> Command {
    Command::new("fake-enosys")
        .author("Aleksa Sarai <cyphar@cyphar.com>")
        .about("Runs a subcommand with certain syscalls disabled (returning -ENOSYS).")
        .arg(
            Arg::new("syscalls")
                .long("syscall")
                .short('s')
                .help("syscall name (or number) to mask (comma-separated or passed multiple times)")
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("command")
                .help("the subcommand to run")
                .allow_hyphen_values(true)
                .num_args(..)
                .trailing_var_arg(true)
                .required(true),
        )
}

#[test]
fn verify_app() {
    cli().debug_assert();
}

fn seccomp_set_filter(mut filter: impl AsMut<[libc::sock_filter]>) -> Result<(), Error> {
    let filter = filter.as_mut();

    let bpf_prog = libc::sock_fprog {
        len: filter.len() as u16,
        filter: filter.as_mut_ptr(),
    };

    // SAFETY: Safe libc function and the kernel copies the filter internally so
    // the lifetime is not an issue.
    let ret = unsafe {
        libc::prctl(
            libc::PR_SET_SECCOMP,
            libc::SECCOMP_MODE_FILTER,
            &bpf_prog as *const _,
        )
    };
    if ret < 0 {
        Err(std::io::Error::last_os_error()).context("failed to install seccomp filter")?;
    }

    Ok(())
}

fn main() -> Result<(), Error> {
    let m = cli().get_matches();

    let syscalls: Vec<Sysno> = m
        .get_many::<String>("syscalls")
        .map(|iter| {
            iter.flat_map(|s| s.split(","))
                .filter(|&s| !s.is_empty())
                .map(|syscall| -> Result<_, Error> {
                    syscall.parse::<Sysno>().or_else(|_| {
                        syscall
                            .parse::<usize>()
                            .map_err(|_| anyhow!("syscall {syscall:?} is not a known syscall"))
                            .and_then(|sysno| {
                                Sysno::new(sysno).ok_or_else(|| {
                                    anyhow!("syscall #{sysno} is not a known syscall")
                                })
                            })
                    })
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .unwrap_or_else(|| Ok(vec![]))?;

    let cmdline: Vec<_> = m
        .get_many::<String>("command")
        .ok_or_else(|| anyhow!("no command found"))?
        .cloned()
        .collect();
    let (prog, args) = cmdline
        .split_first()
        .context("command-line must have at least one element")?;

    if !syscalls.is_empty() {
        let mut filter = bpf::compile_filter(&syscalls)?;

        // Unprivileged processes cannot enable seccomp-bpf unless they also set the
        // no-new-privs bit (to stop them from being able to trick setuid binaries).
        if !rustix_process::getuid().is_root() {
            rustix_thread::set_no_new_privs(true).context("could not set no-new-privs bit")?;
        }

        seccomp_set_filter(&mut filter)?;
    }

    Err(StdCmd::new(prog).args(args).exec()).context("could not exec subcommand")
}
