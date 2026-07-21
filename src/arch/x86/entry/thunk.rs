//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/entry/thunk.S
//! test-origin: linux:vendor/linux/arch/x86/entry/thunk.S
//! x86 entry thunks exported to Linux-built modules.

use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "preempt_schedule_thunk",
        linux_preempt_schedule_thunk as usize,
        true,
    );
    export_symbol_once(
        "preempt_schedule_notrace_thunk",
        linux_preempt_schedule_notrace_thunk as usize,
        true,
    );
}

/// `preempt_schedule_thunk` - `vendor/linux/arch/x86/entry/thunk.S`.
///
/// Linux's x86-64 `THUNK` macro preserves caller-visible argument registers
/// and `rax` around `preempt_schedule`. Linux-built modules can call this from
/// compiler-emitted inline asm while live values are still in those registers.
#[unsafe(naked)]
pub unsafe extern "C" fn linux_preempt_schedule_thunk() {
    core::arch::naked_asm!(
        "push rbp",
        "mov rbp, rsp",
        "push rdi",
        "push rsi",
        "push rdx",
        "push rcx",
        "push rax",
        "push r8",
        "push r9",
        "push r10",
        "push r11",
        "call {body}",
        "pop r11",
        "pop r10",
        "pop r9",
        "pop r8",
        "pop rax",
        "pop rcx",
        "pop rdx",
        "pop rsi",
        "pop rdi",
        "pop rbp",
        "ret",
        body = sym linux_preempt_schedule_body,
    );
}

/// `preempt_schedule_notrace_thunk` - `vendor/linux/arch/x86/entry/thunk.S`.
///
/// Same Linux `THUNK` wrapper as `preempt_schedule_thunk`, targeting the
/// notrace scheduler entry. Lupos does not add tracing around `schedule()`, so
/// the notrace body shares the native scheduler call while preserving the
/// separate Linux module ABI symbol.
#[unsafe(naked)]
pub unsafe extern "C" fn linux_preempt_schedule_notrace_thunk() {
    core::arch::naked_asm!(
        "push rbp",
        "mov rbp, rsp",
        "push rdi",
        "push rsi",
        "push rdx",
        "push rcx",
        "push rax",
        "push r8",
        "push r9",
        "push r10",
        "push r11",
        "call {body}",
        "pop r11",
        "pop r10",
        "pop r9",
        "pop r8",
        "pop rax",
        "pop rcx",
        "pop rdx",
        "pop rsi",
        "pop rdi",
        "pop rbp",
        "ret",
        body = sym linux_preempt_schedule_notrace_body,
    );
}

#[inline(never)]
unsafe extern "C" fn linux_preempt_schedule_body() {
    #[cfg(not(test))]
    if should_preempt_schedule() {
        crate::kernel::locking::local_irq_enable();
        unsafe {
            crate::kernel::sched::schedule();
        }
        crate::kernel::locking::local_irq_enable();
    }
}

#[inline(never)]
unsafe extern "C" fn linux_preempt_schedule_notrace_body() {
    #[cfg(not(test))]
    if should_preempt_schedule() {
        crate::kernel::locking::local_irq_enable();
        unsafe {
            crate::kernel::sched::schedule();
        }
        crate::kernel::locking::local_irq_enable();
    }
}

fn should_preempt_schedule_with(need_resched: bool, preempt_count: u32, in_irq: bool) -> bool {
    need_resched && preempt_count == 0 && !in_irq
}

#[cfg(not(test))]
fn should_preempt_schedule() -> bool {
    let current = unsafe { crate::kernel::sched::get_current() };
    if current.is_null() {
        return false;
    }
    let need_resched = unsafe {
        (*current)
            .thread_info
            .flags
            .load(core::sync::atomic::Ordering::Acquire)
            & crate::kernel::task::TIF_NEED_RESCHED
            != 0
    };
    should_preempt_schedule_with(
        need_resched,
        crate::kernel::locking::preempt::preempt_count(),
        crate::kernel::locking::preempt::in_irq(),
    )
}

#[cfg(test)]
fn should_preempt_schedule() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINUX_THUNK_S: &str = include_str!("../../../../vendor/linux/arch/x86/entry/thunk.S");

    #[test]
    fn linux_source_lists_both_schedule_thunks_and_exports() {
        assert!(LINUX_THUNK_S.contains("THUNK preempt_schedule_thunk, preempt_schedule"));
        assert!(
            LINUX_THUNK_S
                .contains("THUNK preempt_schedule_notrace_thunk, preempt_schedule_notrace")
        );
        assert!(LINUX_THUNK_S.contains("EXPORT_SYMBOL(preempt_schedule_thunk)"));
        assert!(LINUX_THUNK_S.contains("EXPORT_SYMBOL(preempt_schedule_notrace_thunk)"));
    }

    #[test]
    fn schedule_thunk_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("preempt_schedule_thunk"),
            Some(linux_preempt_schedule_thunk as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("preempt_schedule_notrace_thunk"),
            Some(linux_preempt_schedule_notrace_thunk as usize)
        );
    }

    #[test]
    fn preempt_schedule_gate_matches_linux_conditions() {
        assert!(should_preempt_schedule_with(true, 0, false));
        assert!(!should_preempt_schedule_with(false, 0, false));
        assert!(!should_preempt_schedule_with(true, 1, false));
        assert!(!should_preempt_schedule_with(true, 0, true));
    }
}
