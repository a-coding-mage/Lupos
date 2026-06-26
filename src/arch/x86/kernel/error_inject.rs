//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! Function-override hook used by the BPF error-injection infrastructure.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/lib/error-inject.c
//!
//! Linux exposes `override_function_with_return()` which, when called from
//! within a kprobe pre-handler attached to an injection-enabled function,
//! rewrites the saved instruction pointer in `pt_regs` to point at a trampoline
//! (`just_return_func`) consisting of a single `RET`. After the kprobe
//! returns, the CPU resumes at the trampoline, which immediately returns to
//! the *caller* of the original function — effectively cancelling the call.
//!
//! Lupos reuses `crate::arch::x86::kernel::ptrace::PtRegs` as the x86_64 `pt_regs`
//! layout (preserves the SYSV/Linux ABI). The trampoline lives in the
//! kernel's `.text` and is exposed via `just_return_func` (`naked_asm!` in
//! Rust because the function must consist of a single RET — no prologue, no
//! frame pointer setup).

use crate::arch::x86::kernel::ptrace::PtRegs;

// Bare `RET` trampoline. Linker symbol mirrors Linux's `just_return_func`.
// On ELF targets (the kernel itself) we emit it via global_asm! with the
// `.type`/`.size` directives the C version uses. Host unit tests provide
// a plain Rust shim so the
// override path is exercisable without ELF-specific assembler syntax.

#[cfg(all(target_os = "none", target_arch = "x86_64"))]
unsafe extern "C" {
    pub fn just_return_func();
}

#[cfg(all(target_os = "none", target_arch = "x86_64"))]
core::arch::global_asm!(
    ".text",
    ".type just_return_func, @function",
    ".globl just_return_func",
    ".p2align 4, 0x90", // ASM_FUNC_ALIGN (16-byte align, NOP fill).
    "just_return_func:",
    "    ret",
    ".size just_return_func, .-just_return_func",
);

/// Host-test shim — a real (Rust) function whose address stands in for
/// the ELF `just_return_func` symbol during unit tests.
#[cfg(not(all(target_os = "none", target_arch = "x86_64")))]
#[inline(never)]
pub extern "C" fn just_return_func() {}

/// Rewrite `regs->ip` so the patched function returns immediately.
///
/// Mirrors `override_function_with_return()` from
/// `vendor/linux/arch/x86/lib/error-inject.c` line for line; the only
/// difference is the trampoline address comes from the Rust extern symbol
/// rather than a C function pointer.
///
/// # Safety
/// Caller must be inside a kprobe pre-handler whose `pt_regs` describes a
/// function annotated `ALLOW_ERROR_INJECTION`. The kprobe core uses
/// `NOKPROBE_SYMBOL(override_function_with_return)` to avoid recursion;
/// callers must preserve that guarantee for Rust by not probing this fn.
pub unsafe fn override_function_with_return(regs: *mut PtRegs) {
    if regs.is_null() {
        return;
    }
    let target = just_return_func as usize as u64;
    unsafe {
        (*regs).rip = target;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a zeroed `PtRegs`, hand it to the override, and confirm `rip`
    /// now points at the trampoline. Mirrors the user-visible contract of
    /// `override_function_with_return()` in error-inject.c.
    #[test]
    fn override_writes_trampoline_address_into_pt_regs_rip() {
        let mut regs: PtRegs = unsafe { core::mem::zeroed() };
        unsafe { override_function_with_return(&mut regs) };
        // On test builds the trampoline lives in the test binary; the
        // address must be non-zero and equal to the symbol's address.
        let expected = just_return_func as usize as u64;
        assert_ne!(expected, 0);
        assert_eq!(regs.rip, expected);
    }

    #[test]
    fn override_is_safe_on_null_regs() {
        // Linux passes pt_regs from kprobes — it's never null in practice,
        // but Rust safety dictates we check. Confirm no UB on a null ptr.
        unsafe { override_function_with_return(core::ptr::null_mut()) };
    }
}
