//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/ia32/audit.c
//! test-origin: linux:vendor/linux/arch/x86/ia32/audit.c
//! IA32 audit syscall classification.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/ia32/audit.c

pub const AUDITSC_NATIVE: u32 = 0;
pub const AUDITSC_COMPAT: u32 = 1;
pub const AUDITSC_OPEN: u32 = 2;
pub const AUDITSC_OPENAT: u32 = 3;
pub const AUDITSC_SOCKETCALL: u32 = 4;
pub const AUDITSC_EXECVE: u32 = 5;
pub const AUDITSC_OPENAT2: u32 = 6;

pub const IA32_NR_OPEN: u32 = 5;
pub const IA32_NR_EXECVE: u32 = 11;
pub const IA32_NR_SOCKETCALL: u32 = 102;
pub const IA32_NR_OPENAT: u32 = 295;
pub const IA32_NR_EXECVEAT: u32 = 358;
pub const IA32_NR_OPENAT2: u32 = 437;

pub const fn ia32_classify_syscall(syscall: u32) -> u32 {
    match syscall {
        IA32_NR_OPEN => AUDITSC_OPEN,
        IA32_NR_OPENAT => AUDITSC_OPENAT,
        IA32_NR_SOCKETCALL => AUDITSC_SOCKETCALL,
        IA32_NR_EXECVE | IA32_NR_EXECVEAT => AUDITSC_EXECVE,
        IA32_NR_OPENAT2 => AUDITSC_OPENAT2,
        _ => AUDITSC_COMPAT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ia32_audit_classifier_matches_linux_special_cases() {
        assert_eq!(ia32_classify_syscall(IA32_NR_OPEN), AUDITSC_OPEN);
        assert_eq!(
            ia32_classify_syscall(IA32_NR_SOCKETCALL),
            AUDITSC_SOCKETCALL
        );
        assert_eq!(ia32_classify_syscall(IA32_NR_EXECVEAT), AUDITSC_EXECVE);
        assert_eq!(ia32_classify_syscall(9999), AUDITSC_COMPAT);
    }
}
