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

use anyhow::{Context, Error};
use syscalls::Sysno;

pub(crate) fn compile_filter(
    syscalls: impl AsRef<[Sysno]>,
) -> Result<Vec<libc::sock_filter>, Error> {
    use bpfvm::{
        asm::{self, Operation::*},
        bpf::{JmpOp::*, Mode::*, Src::*},
        seccomp::{FieldOffset, SeccompReturn},
    };

    // Generate a very basic seccomp-bpf profile:
    asm::compile(
        &vec![
            // TODO: Check that the architecture is native...
            // load [0] (syscall number)
            Load(ABS, FieldOffset::Syscall.offset()),
        ]
        .into_iter()
        // jeq [$sysno1],[ENOSYS]
        // jeq [$sysno2],[ENOSYS]
        // ...
        .chain(
            syscalls
                .as_ref()
                .iter()
                .flat_map(|sysno| Some(Jump(JEQ, sysno.id() as u32, Some("ENOSYS"), None))),
        )
        // ret [0]
        // 'ENOSYS:
        // ret [ENOSYS]
        .chain(vec![
            Label("ALLOW"),
            Return(Const, SeccompReturn::Allow.into()),
            Label("ENOSYS"),
            Return(Const, SeccompReturn::Errno(libc::ENOSYS as u32).into()),
        ])
        .collect::<Vec<_>>(),
    )
    .context("failed to compile seccomp-bpf filter")
}

#[cfg(test)]
#[cfg(target_arch = "x86_64")] // for SECCOMP_ARCH_NATIVE
mod test {
    use super::*;

    use anyhow::Error;
    use bpfvm::{
        seccomp::SeccompReturn,
        vm::{self, BpfVM},
    };
    use pretty_assertions::assert_eq;
    use syscalls::Sysno;

    fn syscall_data(sysno: Sysno) -> libc::seccomp_data {
        libc::seccomp_data {
            nr: sysno.id(),
            arch: SECCOMP_ARCH_NATIVE,
            instruction_pointer: 0xdeadbeefcafe,
            args: [0x0, 0x0, 0x0, 0x0, 0x0, 0x0],
        }
    }

    #[cfg(target_arch = "x86_64")]
    const SECCOMP_ARCH_NATIVE: u32 = bpfvm::seccomp::AUDIT_ARCH_X86_64;

    #[test]
    fn test_single_filter_allow() -> Result<(), Error> {
        let filter = compile_filter([Sysno::openat])?;

        let data = syscall_data(Sysno::link);
        let ret: SeccompReturn = BpfVM::new(&filter)?
            .run(vm::any_to_data(&data))?
            .try_into()?;
        assert_eq!(ret, SeccompReturn::Allow, "seccomp should allow link");

        Ok(())
    }

    #[test]
    fn test_multi_filter_allow() -> Result<(), Error> {
        let filter = compile_filter([Sysno::openat2, Sysno::statx])?;

        let data = syscall_data(Sysno::openat);
        let ret: SeccompReturn = BpfVM::new(&filter)?
            .run(vm::any_to_data(&data))?
            .try_into()?;
        assert_eq!(ret, SeccompReturn::Allow, "seccomp should allow openat");

        Ok(())
    }

    #[test]
    fn test_single_filter_enosys() -> Result<(), Error> {
        let filter = compile_filter([Sysno::openat2])?;

        let data = syscall_data(Sysno::openat2);
        let ret: SeccompReturn = BpfVM::new(&filter)?
            .run(vm::any_to_data(&data))?
            .try_into()?;
        assert_eq!(
            ret,
            SeccompReturn::Errno(libc::ENOSYS as u32),
            "errno should be ENOSYS for statx"
        );

        let data = syscall_data(Sysno::openat);
        let ret: SeccompReturn = BpfVM::new(&filter)?
            .run(vm::any_to_data(&data))?
            .try_into()?;
        assert_eq!(ret, SeccompReturn::Allow, "seccomp should allow openat");

        Ok(())
    }

    #[test]
    fn test_multi_filter_enosys() -> Result<(), Error> {
        let filter = compile_filter([Sysno::openat2, Sysno::statx])?;

        let data = syscall_data(Sysno::statx);
        let ret: SeccompReturn = BpfVM::new(&filter)?
            .run(vm::any_to_data(&data))?
            .try_into()?;
        assert_eq!(
            ret,
            SeccompReturn::Errno(libc::ENOSYS as u32),
            "errno should be ENOSYS for statx"
        );

        let data = syscall_data(Sysno::openat2);
        let ret: SeccompReturn = BpfVM::new(&filter)?
            .run(vm::any_to_data(&data))?
            .try_into()?;
        assert_eq!(
            ret,
            SeccompReturn::Errno(libc::ENOSYS as u32),
            "errno should be ENOSYS for statx"
        );

        Ok(())
    }
}
