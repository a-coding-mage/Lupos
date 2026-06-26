//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/audit_64.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/audit_64.c
//! x86-64 syscall audit classification.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/audit_64.c

pub const AUDIT_ARCH_LE: u32 = 0x4000_0000;
pub const AUDIT_ARCH_64BIT: u32 = 0x8000_0000;
pub const EM_386: u32 = 3;
pub const EM_X86_64: u32 = 62;

pub const AUDIT_ARCH_I386: u32 = EM_386 | AUDIT_ARCH_LE;
pub const AUDIT_ARCH_X86_64: u32 = EM_X86_64 | AUDIT_ARCH_LE | AUDIT_ARCH_64BIT;

pub const AUDITSC_NATIVE: u32 = 0;
pub const AUDITSC_COMPAT: u32 = 1;
pub const AUDITSC_OPEN: u32 = 2;
pub const AUDITSC_OPENAT: u32 = 3;
pub const AUDITSC_EXECVE: u32 = 5;
pub const AUDITSC_OPENAT2: u32 = 6;

pub const __NR_OPEN: u32 = 2;
pub const __NR_EXECVE: u32 = 59;
pub const __NR_OPENAT: u32 = 257;
pub const __NR_EXECVEAT: u32 = 322;
pub const __NR_OPENAT2: u32 = 437;

pub const I386_NR_OPEN: u32 = 5;
pub const I386_NR_EXECVE: u32 = 11;
pub const I386_NR_OPENAT: u32 = 295;
pub const I386_NR_EXECVEAT: u32 = 358;
pub const I386_NR_OPENAT2: u32 = 437;

pub const fn audit_classify_arch(arch: u32, ia32_emulation: bool) -> u32 {
    if ia32_emulation && arch == AUDIT_ARCH_I386 {
        AUDITSC_COMPAT
    } else {
        AUDITSC_NATIVE
    }
}

pub const fn audit_classify_syscall(abi: u32, syscall: u32, ia32_emulation: bool) -> u32 {
    if ia32_emulation && abi == AUDITSC_COMPAT {
        match syscall {
            I386_NR_OPEN => AUDITSC_OPEN,
            I386_NR_OPENAT => AUDITSC_OPENAT,
            I386_NR_OPENAT2 => AUDITSC_OPENAT2,
            I386_NR_EXECVE | I386_NR_EXECVEAT => AUDITSC_EXECVE,
            _ => AUDITSC_COMPAT,
        }
    } else {
        match syscall {
            __NR_OPEN => AUDITSC_OPEN,
            __NR_OPENAT => AUDITSC_OPENAT,
            __NR_OPENAT2 => AUDITSC_OPENAT2,
            __NR_EXECVE | __NR_EXECVEAT => AUDITSC_EXECVE,
            _ => AUDITSC_NATIVE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_arch_constants_match_linux_uapi() {
        assert_eq!(AUDIT_ARCH_I386, 0x4000_0003);
        assert_eq!(AUDIT_ARCH_X86_64, 0xc000_003e);
    }

    #[test]
    fn arch_classifier_tracks_ia32_emulation() {
        assert_eq!(audit_classify_arch(AUDIT_ARCH_I386, true), AUDITSC_COMPAT);
        assert_eq!(audit_classify_arch(AUDIT_ARCH_I386, false), AUDITSC_NATIVE);
        assert_eq!(audit_classify_arch(AUDIT_ARCH_X86_64, true), AUDITSC_NATIVE);
    }

    #[test]
    fn syscall_classifier_handles_native_special_classes() {
        assert_eq!(
            audit_classify_syscall(AUDITSC_NATIVE, __NR_OPEN, true),
            AUDITSC_OPEN
        );
        assert_eq!(
            audit_classify_syscall(AUDITSC_NATIVE, __NR_EXECVEAT, true),
            AUDITSC_EXECVE
        );
        assert_eq!(
            audit_classify_syscall(AUDITSC_NATIVE, __NR_OPENAT2, true),
            AUDITSC_OPENAT2
        );
    }

    #[test]
    fn syscall_classifier_handles_compat_when_enabled() {
        assert_eq!(
            audit_classify_syscall(AUDITSC_COMPAT, I386_NR_OPENAT, true),
            AUDITSC_OPENAT
        );
        assert_eq!(
            audit_classify_syscall(AUDITSC_COMPAT, I386_NR_EXECVE, true),
            AUDITSC_EXECVE
        );
    }
}
