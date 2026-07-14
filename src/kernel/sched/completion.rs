//! linux-parity: complete
//! linux-source: vendor/linux/kernel/sched/completion.c
//! test-origin: linux:vendor/linux/kernel/sched/completion.c
//! Scheduler completion primitive.
//!
//! Mirrors `vendor/linux/kernel/sched/completion.c`. The implementation lives
//! in `kernel::locking::completion`; this module keeps the scheduler namespace
//! Linux-compatible and provides the C-style helper names.

use core::ffi::c_void;

pub use crate::kernel::locking::completion::Completion;
use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("complete", linux_complete as usize, false);
    export_symbol_once("complete_all", linux_complete_all as usize, false);
    export_symbol_once(
        "wait_for_completion",
        linux_wait_for_completion as usize,
        false,
    );
    export_symbol_once(
        "try_wait_for_completion",
        linux_try_wait_for_completion_export as usize,
        false,
    );
    export_symbol_once(
        "wait_for_completion_interruptible",
        linux_wait_for_completion_interruptible as usize,
        false,
    );
    export_symbol_once(
        "wait_for_completion_interruptible_timeout",
        linux_wait_for_completion_interruptible_timeout as usize,
        false,
    );
}

pub fn init_completion(completion: &Completion) {
    completion.reinit();
}

pub fn reinit_completion(completion: &Completion) {
    completion.reinit();
}

pub fn complete(completion: &Completion) {
    completion.complete();
}

pub fn complete_all(completion: &Completion) {
    completion.complete_all();
}

pub fn wait_for_completion(completion: &Completion) {
    completion.wait();
}

pub fn try_wait_for_completion(completion: &Completion) -> bool {
    completion.try_wait()
}

unsafe fn linux_completion_done_ptr(completion: *mut c_void) -> Option<*mut u32> {
    (!completion.is_null()).then_some(completion.cast::<u32>())
}

/// Try to consume one module ABI `struct completion` signal.
///
/// Vendor Linux modules pass their own `struct completion` layout, whose first
/// field is the `done` counter.  Keep this helper raw-layout based instead of
/// casting to Lupos' native `Completion`, whose wait-list representation is
/// intentionally different.
pub unsafe fn linux_try_wait_for_completion_raw(completion: *mut c_void) -> bool {
    let Some(done) = (unsafe { linux_completion_done_ptr(completion) }) else {
        return false;
    };
    let current = unsafe { core::ptr::read_volatile(done) };
    if current == 0 {
        return false;
    }
    if current != u32::MAX {
        unsafe {
            core::ptr::write_volatile(done, current - 1);
        }
    }
    true
}

unsafe fn linux_wait_for_completion_jiffies(completion: *mut c_void, timeout: u64) -> u64 {
    if completion.is_null() {
        return 0;
    }
    if unsafe { linux_try_wait_for_completion_raw(completion) } {
        return timeout.max(1);
    }
    if timeout == 0 {
        return 0;
    }

    let expires = crate::kernel::time::jiffies::jiffies().saturating_add(timeout);
    loop {
        #[cfg(not(test))]
        {
            // Match `linux_wait_for_completion`: raw module completions do not
            // enqueue on Lupos' native wait queues, so pump vendor-driver events
            // at the wait boundary before yielding.
            let _ = crate::linux_driver_abi::poll_driver_abi_events();
            unsafe {
                crate::kernel::sched::schedule_with_irqs_enabled();
            }
        }
        #[cfg(test)]
        return 0;

        if unsafe { linux_try_wait_for_completion_raw(completion) } {
            return expires
                .saturating_sub(crate::kernel::time::jiffies::jiffies())
                .max(1);
        }
        if crate::kernel::time::jiffies::jiffies() >= expires {
            return 0;
        }
    }
}

/// `complete` - `vendor/linux/kernel/sched/completion.c:36`.
#[unsafe(export_name = "complete")]
pub unsafe extern "C" fn linux_complete(completion: *mut c_void) {
    let Some(done) = (unsafe { linux_completion_done_ptr(completion) }) else {
        return;
    };
    let current = unsafe { core::ptr::read_volatile(done) };
    unsafe {
        if current != u32::MAX {
            core::ptr::write_volatile(done, current.saturating_add(1));
        }
    }
}

/// `complete_all` - `vendor/linux/kernel/sched/completion.c:59`.
#[unsafe(export_name = "complete_all")]
pub unsafe extern "C" fn linux_complete_all(completion: *mut c_void) {
    let Some(done) = (unsafe { linux_completion_done_ptr(completion) }) else {
        return;
    };
    unsafe {
        core::ptr::write_volatile(done, u32::MAX);
    }
}

/// `wait_for_completion` - `vendor/linux/kernel/sched/completion.c:139`.
#[unsafe(export_name = "wait_for_completion")]
pub unsafe extern "C" fn linux_wait_for_completion(completion: *mut c_void) {
    loop {
        if unsafe { linux_try_wait_for_completion_raw(completion) } {
            return;
        }
        #[cfg(not(test))]
        {
            // Pump driver-ABI completions directly. The waiter busy-loops in the
            // caller's context (it never sets TASK sleeping), so the scheduler
            // never goes idle and the idle-path driver pump cannot run — on a
            // multi-CPU boot that means an AHCI/libata completion (e.g. IDENTIFY,
            // issued via ata_exec_internal → wait_for_completion) would never be
            // reaped and this loop would spin forever. Reaping here mirrors the
            // block-facade wait loop.
            let _ = crate::linux_driver_abi::poll_driver_abi_events();
            unsafe {
                crate::kernel::sched::schedule_with_irqs_enabled();
            }
        }
        #[cfg(test)]
        return;
    }
}

/// `try_wait_for_completion` - `vendor/linux/kernel/sched/completion.c:309`.
pub unsafe extern "C" fn linux_try_wait_for_completion_export(completion: *mut c_void) -> bool {
    unsafe { linux_try_wait_for_completion_raw(completion) }
}

/// `wait_for_completion_interruptible` - `vendor/linux/kernel/sched/completion.c:219`.
pub unsafe extern "C" fn linux_wait_for_completion_interruptible(completion: *mut c_void) -> i32 {
    unsafe {
        linux_wait_for_completion(completion);
    }
    0
}

/// `wait_for_completion_interruptible_timeout` - `vendor/linux/kernel/sched/completion.c:241`.
pub unsafe extern "C" fn linux_wait_for_completion_interruptible_timeout(
    completion: *mut c_void,
    timeout: u64,
) -> i64 {
    // Lupos does not deliver Linux task signals into this raw module wait path,
    // so there is no `-ERESTARTSYS` case here; completion and timeout semantics
    // match Linux's positive/zero return values.
    unsafe { linux_wait_for_completion_jiffies(completion, timeout) as i64 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduler_completion_helpers_round_trip() {
        let c = Completion::new();
        complete(&c);
        assert!(try_wait_for_completion(&c));
        assert!(!try_wait_for_completion(&c));
        complete_all(&c);
        assert!(try_wait_for_completion(&c));
    }

    #[test]
    fn linux_completion_module_exports_register() {
        register_module_exports();

        assert_eq!(
            crate::kernel::module::find_symbol("complete"),
            Some(linux_complete as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("wait_for_completion"),
            Some(linux_wait_for_completion as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("try_wait_for_completion"),
            Some(linux_try_wait_for_completion_export as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("wait_for_completion_interruptible"),
            Some(linux_wait_for_completion_interruptible as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("wait_for_completion_interruptible_timeout"),
            Some(linux_wait_for_completion_interruptible_timeout as usize)
        );
    }

    #[test]
    fn linux_completion_done_counter_matches_basic_flow() {
        let mut raw = 0u32;
        unsafe {
            linux_complete((&mut raw as *mut u32).cast());
            assert_eq!(raw, 1);
            linux_wait_for_completion((&mut raw as *mut u32).cast());
            assert_eq!(raw, 0);
            linux_complete_all((&mut raw as *mut u32).cast());
            assert_eq!(raw, u32::MAX);
            linux_wait_for_completion((&mut raw as *mut u32).cast());
            assert_eq!(raw, u32::MAX);
        }
    }

    #[test]
    fn linux_completion_interruptible_timeout_returns_remaining_jiffies() {
        let mut raw = 1u32;
        unsafe {
            assert_eq!(
                linux_wait_for_completion_interruptible_timeout((&mut raw as *mut u32).cast(), 7),
                7
            );
            assert_eq!(raw, 0);

            assert_eq!(
                linux_wait_for_completion_interruptible_timeout((&mut raw as *mut u32).cast(), 0),
                0
            );
        }
    }
}
