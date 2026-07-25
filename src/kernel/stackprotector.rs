//! linux-parity: partial
//! linux-source: vendor/linux/kernel/panic.c
//! test-origin: linux:vendor/linux/lib/test_stackinit.c
//! GCC/Clang kernel stack-protector ABI for vendor-built modules.

use crate::kernel::module::{export_symbol, find_symbol};
use crate::log_error;

/// LLVM's global-guard ABI for the bare-metal Rust kernel.
///
/// The 64-bit boot entry randomizes this object before its first call into
/// stack-protected Rust.  Vendor C modules use their separate Linux per-CPU
/// `%gs:__ref_stack_chk_guard` ABI and therefore remain byte-for-byte
/// unchanged.
#[unsafe(no_mangle)]
pub static mut __stack_chk_guard: usize = 0x6c75_706f_735f_7300;

pub fn register_module_exports() {
    if find_symbol("__stack_chk_fail").is_none() {
        export_symbol("__stack_chk_fail", __stack_chk_fail as usize, false);
    }
    if find_symbol("__stack_chk_guard").is_none() {
        export_symbol(
            "__stack_chk_guard",
            core::ptr::addr_of!(__stack_chk_guard) as usize,
            false,
        );
    }
    if find_symbol("__ref_stack_chk_guard").is_none() {
        export_symbol(
            "__ref_stack_chk_guard",
            crate::arch::x86::kernel::setup_percpu::stack_chk_guard_symbol(),
            false,
        );
    }
}

/// Called by compiler-generated epilogues after a canary mismatch.
///
/// Keep this trampoline naked: the failing function's continuation is at the
/// original `[rsp]`, before any Rust prologue can move the stack.  The regular
/// implementation below can then log that return PC without treating its own
/// frame as the failing caller's frame.
#[unsafe(naked)]
#[unsafe(export_name = "__stack_chk_fail")]
pub unsafe extern "C" fn __stack_chk_fail() -> ! {
    core::arch::naked_asm!(
        "mov rdi, [rsp]",
        "jmp {impl}",
        impl = sym stack_chk_fail_impl,
    )
}

#[inline(never)]
extern "C" fn stack_chk_fail_impl(caller: usize) -> ! {
    let sp: usize;
    unsafe {
        core::arch::asm!("mov {}, rsp", out(reg) sp, options(nomem, preserves_flags));
        log_error!(
            "stack-protector",
            "stack-protector: canary caller={:#018x} rsp={:#018x}",
            caller,
            sp,
        );
    }
    panic!("stack-protector: Kernel stack is corrupted")
}
