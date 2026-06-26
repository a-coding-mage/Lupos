//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/entry
//! linux-source: vendor/linux/kernel/sys_ni.c
//! Catch-all syscall returning -ENOSYS.
//!
//! Faithful port of Linux `sys_ni_syscall()` (vendor/linux/kernel/sys_ni.c:20),
//! whose entire body is `return -ENOSYS;` (ENOSYS == 38). With
//! CONFIG_ARCH_HAS_SYSCALL_WRAPPER the x86-64 alias is
//! `__x64_sys_ni_syscall(const struct pt_regs *)`, matching our
//! `(*mut PtRegs) -> i64` signature. Linux's `COND_SYSCALL(...)` weak-alias list
//! is realized in Lupos by routing unimplemented `SYS_CALL_TABLE` slots here.

use crate::arch::x86::kernel::ptrace::PtRegs;

pub unsafe extern "C" fn sys_ni_syscall(_regs: *mut PtRegs) -> i64 {
    -38 // ENOSYS
}
