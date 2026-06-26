//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! x86 compatibility entry points and non-native ABI gates.
//!
//! Native x86-64 syscall ABI is complete (see entry/syscall.rs). The IA32,
//! vm86, vDSO32, vsyscall64, FRED, and UML compat entry families are real Linux
//! configs that Lupos does not implement; this module gates them fail-closed
//! with the correct errno instead of pretending they exist. Remaining work vs
//! Linux for `complete`: actually implementing those compat entry paths.
//!
//! The native syscall path is `syscall_64`. The 32-bit, vm86, vDSO32, UML, and
//! FRED compatibility entry families are separate Linux configs and are not
//! wired into the Lupos boot target yet. These helpers provide concrete,
//! fail-closed behavior for callers instead of silently pretending the entry
//! path exists.

use crate::include::uapi::errno::{ENODEV, ENOSYS, EOPNOTSUPP};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompatEntry {
    Ia32Syscall,
    Vm86,
    Vdso32,
    Vsyscall64,
    Fred,
    UserModeLinux,
}

pub const fn compat_entry_enabled(entry: CompatEntry) -> bool {
    match entry {
        CompatEntry::Vsyscall64 => false,
        CompatEntry::Ia32Syscall
        | CompatEntry::Vm86
        | CompatEntry::Vdso32
        | CompatEntry::Fred
        | CompatEntry::UserModeLinux => false,
    }
}

pub const fn compat_entry_errno(entry: CompatEntry) -> i32 {
    match entry {
        CompatEntry::Ia32Syscall | CompatEntry::Vm86 => ENOSYS,
        CompatEntry::Vdso32 | CompatEntry::Vsyscall64 | CompatEntry::Fred => EOPNOTSUPP,
        CompatEntry::UserModeLinux => ENODEV,
    }
}

/// Linux keeps the native x86-64 syscall ABI separate from IA32/audit glue.
pub const fn native_syscall_abi() -> &'static str {
    "x86_64"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compat_entries_are_explicitly_disabled() {
        for entry in [
            CompatEntry::Ia32Syscall,
            CompatEntry::Vm86,
            CompatEntry::Vdso32,
            CompatEntry::Vsyscall64,
            CompatEntry::Fred,
            CompatEntry::UserModeLinux,
        ] {
            assert!(!compat_entry_enabled(entry));
            assert!(compat_entry_errno(entry) > 0);
        }
    }

    #[test]
    fn native_abi_remains_x86_64() {
        assert_eq!(native_syscall_abi(), "x86_64");
    }
}
