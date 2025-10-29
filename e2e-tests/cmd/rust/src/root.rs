// SPDX-License-Identifier: MPL-2.0
/*
 * libpathrs: safe path resolution on Linux
 * Copyright (C) 2019-2025 Aleksa Sarai <cyphar@cyphar.com>
 * Copyright (C) 2019-2025 SUSE LLC
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::utils::{self, ModeParser};
use pathrs::{
    flags::{OpenFlags, RenameFlags},
    InodeType, Root, RootRef,
};

use std::{fs::Permissions, path::PathBuf};

use anyhow::{anyhow, Error};
use clap::{Arg, ArgAction, ArgMatches, Command};
use rustix::fs as rustix_fs;

fn resolve_cli() -> Command {
    Command::new("resolve")
        .about("resolve a path inside the root")
        .arg(utils::oflags_arg("reopen", "reopen the handle with these O_* flags").long("reopen"))
        .args(utils::toggle_arg("follow", "follow trailing symlinks"))
        .arg(
            utils::subpath_arg("subpath")
                .help("path inside the root")
                .required(true),
        )
}

fn resolve(root: RootRef<'_>, matches: &ArgMatches) -> Result<(), Error> {
    let subpath = matches
        .get_one::<PathBuf>("subpath")
        .expect("subpath should be set");
    let follow = *matches
        .get_one::<bool>("follow")
        .expect("follow should be set");

    let handle = if follow {
        root.resolve(subpath)
    } else {
        root.resolve_nofollow(subpath)
    }?;
    utils::print_handle(&handle)?;

    if let Some(&oflags) = matches.get_one::<OpenFlags>("reopen") {
        let f = handle.reopen(oflags)?;
        utils::print_file(&f)?;
    }

    Ok(())
}

fn open_cli() -> Command {
    Command::new("open")
        .about("open a path inside the root")
        .arg(
            utils::oflags_arg("oflags", "O_* flags to use when opening the file")
                .long("oflags")
                .default_value("O_RDONLY"),
        )
        .args(utils::toggle_arg("follow", "follow trailing symlinks"))
        .arg(
            utils::subpath_arg("subpath")
                .help("path inside the root")
                .required(true),
        )
}

fn open(root: RootRef<'_>, matches: &ArgMatches) -> Result<(), Error> {
    let subpath = matches
        .get_one::<PathBuf>("subpath")
        .expect("subpath should be set");
    let follow = *matches
        .get_one::<bool>("follow")
        .expect("follow should be set");
    let mut oflags = *matches
        .get_one::<OpenFlags>("oflags")
        .expect("oflags should always be set");

    if !follow {
        oflags.insert(OpenFlags::O_NOFOLLOW);
    }

    let f = root.open_subpath(subpath, oflags)?;
    utils::print_file(&f)?;
    Ok(())
}

fn mkfile_cli() -> Command {
    Command::new("mkfile")
        .about("make an empty file inside the root")
        .arg(
            utils::oflags_arg("oflags", "O_* flags to use when creating the file")
                .long("oflags")
                .default_value("O_RDONLY"),
        )
        .arg(
            Arg::new("mode")
                .long("mode")
                .help("file mode for the created file")
                .default_value("0o644")
                .value_parser(ModeParser),
        )
        .arg(
            utils::subpath_arg("subpath")
                .help("path inside the root")
                .required(true),
        )
}

fn mkfile(root: RootRef<'_>, matches: &ArgMatches) -> Result<(), Error> {
    let subpath = matches
        .get_one::<PathBuf>("subpath")
        .expect("subpath should be set");
    let oflags = *matches
        .get_one::<OpenFlags>("oflags")
        .expect("oflags should always be set");
    let perm = matches
        .get_one::<Permissions>("mode")
        .expect("mode should always be set");

    let f = root.create_file(subpath, oflags, perm)?;
    utils::print_file(&f)?;
    Ok(())
}

fn mkdir_cli() -> Command {
    Command::new("mkdir")
        .about("make an empty directory inside the root")
        .arg(
            Arg::new("mode")
                .long("mode")
                .help("file mode for the created directory")
                .default_value("0o755")
                .value_parser(ModeParser),
        )
        .arg(
            utils::subpath_arg("subpath")
                .help("path inside the root")
                .required(true),
        )
}

fn mkdir(root: RootRef<'_>, matches: &ArgMatches) -> Result<(), Error> {
    let subpath = matches
        .get_one::<PathBuf>("subpath")
        .expect("subpath should be set");
    let perm = matches
        .get_one::<Permissions>("mode")
        .expect("mode should always be set")
        .clone();

    root.create(subpath, &InodeType::Directory(perm))
        .map_err(Into::into)
}

fn mkdir_all_cli() -> Command {
    Command::new("mkdir-all")
        .about("make a directory (including parents) inside the root")
        .arg(
            Arg::new("mode")
                .long("mode")
                .help("file mode for the created directories")
                .default_value("0o755")
                .value_parser(ModeParser),
        )
        .arg(
            utils::subpath_arg("subpath")
                .help("path inside the root")
                .required(true),
        )
}

fn mkdir_all(root: RootRef<'_>, matches: &ArgMatches) -> Result<(), Error> {
    let subpath = matches
        .get_one::<PathBuf>("subpath")
        .expect("subpath should be set");
    let perm = matches
        .get_one::<Permissions>("mode")
        .expect("mode should always be set")
        .clone();

    let handle = root.mkdir_all(subpath, &perm)?;
    utils::print_handle(&handle)?;
    Ok(())
}

fn mknod_cli() -> Command {
    Command::new("mknod")
        .about("make an inode inside the root")
        .arg(
            Arg::new("mode")
                .long("mode")
                .help("created inode mode")
                .default_value("0o644")
                .value_parser(ModeParser),
        )
        .arg(
            utils::subpath_arg("subpath")
                .help("path inside the root")
                .required(true),
        )
        .arg(
            Arg::new("type")
                .help("inode type to create (like mknod(1))")
                .value_parser(["f", "d", "p", "b", "c", "u"])
                .required(true),
        )
        .arg(
            Arg::new("major")
                .help("the major number of the device (for 'b' and 'c'/'u' types)")
                .default_value("0")
                .value_parser(0..=u32::MAX as i64),
        )
        .arg(
            Arg::new("minor")
                .help("the minor number of the device (for 'b' and 'c'/'u' types)")
                .default_value("0")
                .value_parser(0..=u32::MAX as i64),
        )
}

fn mknod(root: RootRef<'_>, matches: &ArgMatches) -> Result<(), Error> {
    let subpath = matches
        .get_one::<PathBuf>("subpath")
        .expect("subpath should be set");
    let perm = matches
        .get_one::<Permissions>("mode")
        .expect("mode should always be set")
        .clone();

    let inode_type = matches
        .get_one::<String>("type")
        .expect("type should always be set");
    let dev = rustix_fs::makedev(
        *matches
            .get_one::<i64>("major")
            .expect("major should always be set") as u32,
        *matches
            .get_one::<i64>("minor")
            .expect("minor should always be set") as u32,
    );

    match inode_type.as_str() {
        "f" => root.create(subpath, &InodeType::File(perm)),
        "d" => root.create(subpath, &InodeType::Directory(perm)),
        "p" => root.create(subpath, &InodeType::Fifo(perm)),
        "b" => root.create(subpath, &InodeType::BlockDevice(perm, dev)),
        "c" | "u" => root.create(subpath, &InodeType::CharacterDevice(perm, dev)),
        _ => unreachable!("guaranteed to not be reachable by value_parser"),
    }
    .map_err(Into::into)
}

fn hardlink_cli() -> Command {
    Command::new("hardlink")
        .about("make a hardlink inside the root")
        .arg(
            utils::subpath_arg("target")
                .help("target path of the hardlink inside the root (must already exist)")
                .required(true),
        )
        .arg(
            utils::subpath_arg("linkname")
                .help("path inside the root for the new hardlink")
                .required(true),
        )
}

fn hardlink(root: RootRef<'_>, matches: &ArgMatches) -> Result<(), Error> {
    let target = matches
        .get_one::<PathBuf>("target")
        .expect("target should be set");
    let linkname = matches
        .get_one::<PathBuf>("linkname")
        .expect("linkname should be set");

    root.create(linkname, &InodeType::Hardlink(target.clone()))
        .map_err(Into::into)
}

fn symlink_cli() -> Command {
    Command::new("symlink")
        .about("make a symbolic link inside the root")
        .arg(
            utils::subpath_arg("target")
                .help("target path of the symlink")
                .required(true),
        )
        .arg(
            utils::subpath_arg("linkname")
                .help("path inside the root for the new symlink")
                .required(true),
        )
}

fn symlink(root: RootRef<'_>, matches: &ArgMatches) -> Result<(), Error> {
    let target = matches
        .get_one::<PathBuf>("target")
        .expect("target should be set");
    let linkname = matches
        .get_one::<PathBuf>("linkname")
        .expect("linkname should be set");

    root.create(linkname, &InodeType::Symlink(target.clone()))
        .map_err(Into::into)
}

fn readlink_cli() -> Command {
    Command::new("readlink")
        .about("read the target path of a symbolic link inside the root")
        .arg(
            utils::subpath_arg("subpath")
                .help("path inside the root")
                .required(true),
        )
}

fn readlink(root: RootRef<'_>, matches: &ArgMatches) -> Result<(), Error> {
    let subpath = matches
        .get_one::<PathBuf>("subpath")
        .expect("subpath should be set");

    let link_target = root.readlink(subpath)?;
    println!("LINK-TARGET {}", link_target.to_string_lossy());
    Ok(())
}

fn unlink_cli() -> Command {
    Command::new("unlink")
        .about("remove a file inside the root")
        .arg(
            utils::subpath_arg("subpath")
                .help("path inside the root")
                .required(true),
        )
}

fn unlink(root: RootRef<'_>, matches: &ArgMatches) -> Result<(), Error> {
    let subpath = matches
        .get_one::<PathBuf>("subpath")
        .expect("subpath should be set");

    root.remove_file(subpath).map_err(Into::into)
}

fn rmdir_cli() -> Command {
    Command::new("rmdir")
        .about("remove an (empty) directory inside the root")
        .arg(
            utils::subpath_arg("subpath")
                .help("path inside the root")
                .required(true),
        )
}

fn rmdir(root: RootRef<'_>, matches: &ArgMatches) -> Result<(), Error> {
    let subpath = matches
        .get_one::<PathBuf>("subpath")
        .expect("subpath should be set");

    root.remove_dir(subpath).map_err(Into::into)
}

fn rmdir_all_cli() -> Command {
    Command::new("rmdir-all")
        .about("remove a path (recursively) inside the root")
        .arg(
            utils::subpath_arg("subpath")
                .help("path inside the root")
                .required(true),
        )
}

fn rmdir_all(root: RootRef<'_>, matches: &ArgMatches) -> Result<(), Error> {
    let subpath = matches
        .get_one::<PathBuf>("subpath")
        .expect("subpath should be set");

    root.remove_all(subpath).map_err(Into::into)
}

fn rename_cli() -> Command {
    Command::new("rename")
        .about("rename a path inside the root")
        .args(utils::toggle_arg(
            "clobber",
            "allow rename target to be clobbered",
        ))
        .arg(
            Arg::new("whiteout")
                .long("whiteout")
                .help("create whiteout inode in place of source")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("exchange")
                .long("exchange")
                .help("swap source and destination inodes")
                .action(ArgAction::SetTrue),
        )
        .arg(
            utils::subpath_arg("source")
                .help("source path inside the root")
                .required(true),
        )
        .arg(
            utils::subpath_arg("destination")
                .help("destination path inside the root")
                .required(true),
        )
}

fn rename(root: RootRef<'_>, matches: &ArgMatches) -> Result<(), Error> {
    let src = matches
        .get_one::<PathBuf>("source")
        .expect("source should be set");
    let dst = matches
        .get_one::<PathBuf>("destination")
        .expect("destination should be set");

    let rename_args = {
        let clobber = *matches
            .get_one::<bool>("clobber")
            .expect("clobber should be set");
        let whiteout = *matches
            .get_one::<bool>("whiteout")
            .expect("whiteout should be set");
        let exchange = *matches
            .get_one::<bool>("exchange")
            .expect("exchange should be set");

        let mut rename_args = RenameFlags::empty();
        if !clobber {
            rename_args.insert(RenameFlags::RENAME_NOREPLACE);
        }
        if whiteout {
            rename_args.insert(RenameFlags::RENAME_WHITEOUT);
        }
        if exchange {
            rename_args.insert(RenameFlags::RENAME_EXCHANGE);
        }
        rename_args
    };

    root.rename(src, dst, rename_args).map_err(Into::into)
}

pub(crate) fn cli() -> Command {
    Command::new("root")
        .about("Root::* operations")
        .arg(
            Arg::new("root")
                .long("root")
                .value_parser(clap::value_parser!(PathBuf))
                .required(true),
        )
        .subcommand(resolve_cli())
        .subcommand(open_cli())
        .subcommand(mkfile_cli())
        .subcommand(mkdir_cli())
        .subcommand(mkdir_all_cli())
        .subcommand(mknod_cli())
        .subcommand(hardlink_cli())
        .subcommand(symlink_cli())
        .subcommand(readlink_cli())
        .subcommand(unlink_cli())
        .subcommand(rmdir_cli())
        .subcommand(rmdir_all_cli())
        .subcommand(rename_cli())
}

pub(crate) fn subcommand(matches: &ArgMatches) -> Result<(), Error> {
    let root = Root::open(
        matches
            .get_one::<PathBuf>("root")
            .expect("root should be set"),
    )?;

    match matches.subcommand() {
        Some(("resolve", sub_matches)) => resolve(root.as_ref(), sub_matches),
        Some(("open", sub_matches)) => open(root.as_ref(), sub_matches),
        Some(("mkfile", sub_matches)) => mkfile(root.as_ref(), sub_matches),
        Some(("mkdir", sub_matches)) => mkdir(root.as_ref(), sub_matches),
        Some(("mkdir-all", sub_matches)) => mkdir_all(root.as_ref(), sub_matches),
        Some(("mknod", sub_matches)) => mknod(root.as_ref(), sub_matches),
        Some(("hardlink", sub_matches)) => hardlink(root.as_ref(), sub_matches),
        Some(("symlink", sub_matches)) => symlink(root.as_ref(), sub_matches),
        Some(("readlink", sub_matches)) => readlink(root.as_ref(), sub_matches),
        Some(("unlink", sub_matches)) => unlink(root.as_ref(), sub_matches),
        Some(("rmdir", sub_matches)) => rmdir(root.as_ref(), sub_matches),
        Some(("rmdir-all", sub_matches)) => rmdir_all(root.as_ref(), sub_matches),
        Some(("rename", sub_matches)) => rename(root.as_ref(), sub_matches),
        Some((subcommand, _)) => {
            // We should never end up here.
            Err(anyhow!("unknown 'root' subcommand '{subcommand}'"))
        }
        None => Err(anyhow!("no 'root' subcommand specified")),
    }
}
