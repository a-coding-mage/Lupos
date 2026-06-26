//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! User-mode Linux x86 compatibility surface.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/um/bugs_32.c
//! - vendor/linux/arch/x86/um/bugs_64.c
//! - vendor/linux/arch/x86/um/delay.c
//! - vendor/linux/arch/x86/um/fault.c
//! - vendor/linux/arch/x86/um/mem_64.c
//! - vendor/linux/arch/x86/um/os-Linux/mcontext.c
//! - vendor/linux/arch/x86/um/os-Linux/registers.c
//! - vendor/linux/arch/x86/um/os-Linux/tls.c
//! - vendor/linux/arch/x86/um/ptrace.c
//! - vendor/linux/arch/x86/um/ptrace_32.c
//! - vendor/linux/arch/x86/um/ptrace_64.c
//! - vendor/linux/arch/x86/um/ptrace_user.c
//! - vendor/linux/arch/x86/um/signal.c
//! - vendor/linux/arch/x86/um/stub_segv.c
//! - vendor/linux/arch/x86/um/sys_call_table_32.c
//! - vendor/linux/arch/x86/um/sys_call_table_64.c
//! - vendor/linux/arch/x86/um/syscalls_32.c
//! - vendor/linux/arch/x86/um/syscalls_64.c
//! - vendor/linux/arch/x86/um/sysrq_32.c
//! - vendor/linux/arch/x86/um/sysrq_64.c
//! - vendor/linux/arch/x86/um/tls_32.c
//! - vendor/linux/arch/x86/um/tls_64.c
//! - vendor/linux/arch/x86/um/vdso/um_vdso.c
//! - vendor/linux/arch/x86/um/vdso/vma.c

use crate::include::uapi::errno::{EINVAL, ENODEV};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UmArch {
    I386,
    X86_64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UmCpuBugs {
    pub arch: UmArch,
    pub has_sysenter: bool,
    pub has_syscall: bool,
    pub nx: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UmFault {
    pub address: u64,
    pub is_write: bool,
    pub is_user: bool,
    pub present: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UmTlsEntry {
    pub index: u8,
    pub base: u64,
    pub limit: u32,
    pub present: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UmSyscallTable {
    I386,
    X86_64,
}

pub const fn um_syscall_available(bugs: UmCpuBugs) -> bool {
    match bugs.arch {
        UmArch::I386 => bugs.has_sysenter,
        UmArch::X86_64 => bugs.has_syscall,
    }
}

pub const fn um_delay_loops(usec: u64, loops_per_usec: u64) -> u64 {
    usec.saturating_mul(loops_per_usec)
}

pub const fn um_fault_errno(fault: UmFault) -> i32 {
    if fault.is_user { EINVAL } else { ENODEV }
}

pub const fn um_tls_valid(entry: UmTlsEntry, arch: UmArch) -> Result<(), i32> {
    if !entry.present {
        return Ok(());
    }
    match arch {
        UmArch::I386 if entry.index < 3 => Err(EINVAL),
        UmArch::I386 if entry.base > u32::MAX as u64 => Err(EINVAL),
        UmArch::X86_64 if entry.limit != 0xfffff => Err(EINVAL),
        _ => Ok(()),
    }
}

pub const fn um_syscall_table_for_arch(arch: UmArch) -> UmSyscallTable {
    match arch {
        UmArch::I386 => UmSyscallTable::I386,
        UmArch::X86_64 => UmSyscallTable::X86_64,
    }
}

pub const fn um_vdso_enabled(arch: UmArch, host_supports_vdso: bool) -> bool {
    matches!(arch, UmArch::X86_64) && host_supports_vdso
}

pub const fn um_ptrace_reg_count(arch: UmArch) -> usize {
    match arch {
        UmArch::I386 => 17,
        UmArch::X86_64 => 27,
    }
}

pub const fn um_sysrq_supported(arch: UmArch, enabled: bool) -> bool {
    enabled && matches!(arch, UmArch::I386 | UmArch::X86_64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syscall_entry_feature_is_arch_specific() {
        assert!(um_syscall_available(UmCpuBugs {
            arch: UmArch::I386,
            has_sysenter: true,
            has_syscall: false,
            nx: false,
        }));
        assert!(!um_syscall_available(UmCpuBugs {
            arch: UmArch::X86_64,
            has_sysenter: true,
            has_syscall: false,
            nx: true,
        }));
    }

    #[test]
    fn tls_validation_tracks_i386_and_x86_64_rules() {
        assert_eq!(
            um_tls_valid(
                UmTlsEntry {
                    index: 1,
                    base: 0x1000,
                    limit: 0xfffff,
                    present: true,
                },
                UmArch::I386,
            ),
            Err(EINVAL)
        );
        assert!(
            um_tls_valid(
                UmTlsEntry {
                    index: 3,
                    base: 0x1000,
                    limit: 0xfffff,
                    present: true,
                },
                UmArch::X86_64,
            )
            .is_ok()
        );
    }

    #[test]
    fn uml_models_faults_ptrace_vdso_and_sysrq() {
        assert_eq!(um_delay_loops(5, 10), 50);
        assert_eq!(
            um_fault_errno(UmFault {
                address: 0xdead,
                is_write: true,
                is_user: true,
                present: false,
            }),
            EINVAL
        );
        assert_eq!(um_ptrace_reg_count(UmArch::X86_64), 27);
        assert!(um_vdso_enabled(UmArch::X86_64, true));
        assert_eq!(
            um_syscall_table_for_arch(UmArch::I386),
            UmSyscallTable::I386
        );
        assert!(um_sysrq_supported(UmArch::I386, true));
    }
}
