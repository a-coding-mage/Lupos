//! linux-parity: partial
//! linux-source: vendor/linux/kernel/irq_work.c
//! Synchronous irq_work compatibility for Linux-built modules.

use core::ffi::c_void;
#[cfg(test)]
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::{AtomicU32, Ordering};

use crate::kernel::module::{export_symbol, find_symbol};

const IRQ_WORK_PENDING: u32 = 0x01;
const IRQ_WORK_BUSY: u32 = 0x02;
const CSD_TYPE_IRQ_WORK: u32 = 0x20;
const IRQ_WORK_CLAIMED: u32 = IRQ_WORK_PENDING | IRQ_WORK_BUSY;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("irq_work_queue", linux_irq_work_queue as usize, true);
    export_symbol_once("irq_work_sync", linux_irq_work_sync as usize, true);
}

#[repr(C)]
struct LinuxCallSingleNode {
    llist_next: *mut c_void,
    flags: u32,
    src: u16,
    dst: u16,
}

#[repr(C)]
struct LinuxIrqWork {
    node: LinuxCallSingleNode,
    func: Option<unsafe extern "C" fn(*mut LinuxIrqWork)>,
}

fn flags(work: *mut LinuxIrqWork) -> &'static AtomicU32 {
    unsafe { &*core::ptr::addr_of_mut!((*work).node.flags).cast::<AtomicU32>() }
}

unsafe fn run_claimed_work(work: *mut LinuxIrqWork) {
    let work_flags = flags(work);
    for _ in 0..64 {
        let mut current = work_flags.load(Ordering::Acquire);
        current &= !IRQ_WORK_PENDING;
        work_flags.store(current, Ordering::Release);

        if let Some(func) = unsafe { (*work).func } {
            unsafe { func(work) };
        }

        let previous = work_flags.fetch_and(!IRQ_WORK_BUSY, Ordering::AcqRel);
        if previous & IRQ_WORK_PENDING == 0 {
            return;
        }
        work_flags.fetch_or(IRQ_WORK_BUSY | CSD_TYPE_IRQ_WORK, Ordering::AcqRel);
    }
}

/// `irq_work_queue` - `vendor/linux/kernel/irq_work.c:116`.
unsafe extern "C" fn linux_irq_work_queue(work: *mut LinuxIrqWork) -> bool {
    if work.is_null() {
        return false;
    }
    let old = flags(work).fetch_or(IRQ_WORK_CLAIMED | CSD_TYPE_IRQ_WORK, Ordering::AcqRel);
    if old & IRQ_WORK_PENDING != 0 {
        return false;
    }
    unsafe { run_claimed_work(work) };
    true
}

/// `irq_work_sync` - `vendor/linux/kernel/irq_work.c:286`.
unsafe extern "C" fn linux_irq_work_sync(work: *mut LinuxIrqWork) {
    if work.is_null() {
        return;
    }
    while flags(work).load(Ordering::Acquire) & IRQ_WORK_BUSY != 0 {
        core::hint::spin_loop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static IRQ_WORK_TEST_CALLS: AtomicUsize = AtomicUsize::new(0);

    unsafe extern "C" fn record_irq_work(_work: *mut LinuxIrqWork) {
        IRQ_WORK_TEST_CALLS.fetch_add(1, Ordering::AcqRel);
    }

    #[test]
    fn exports_irq_work_symbols() {
        register_module_exports();
        assert_eq!(
            find_symbol("irq_work_queue"),
            Some(linux_irq_work_queue as usize)
        );
        assert_eq!(
            find_symbol("irq_work_sync"),
            Some(linux_irq_work_sync as usize)
        );
    }

    #[test]
    fn queue_runs_work_before_sync_returns() {
        IRQ_WORK_TEST_CALLS.store(0, Ordering::Release);
        let mut work = LinuxIrqWork {
            node: LinuxCallSingleNode {
                llist_next: core::ptr::null_mut(),
                flags: 0,
                src: 0,
                dst: 0,
            },
            func: Some(record_irq_work),
        };

        unsafe {
            assert!(linux_irq_work_queue(&mut work));
            linux_irq_work_sync(&mut work);
        }
        assert_eq!(IRQ_WORK_TEST_CALLS.load(Ordering::Acquire), 1);
        assert_eq!(work.node.flags & IRQ_WORK_BUSY, 0);
        assert_eq!(work.node.flags & IRQ_WORK_PENDING, 0);
    }
}
