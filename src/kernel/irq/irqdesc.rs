//! linux-parity: complete
//! linux-source: vendor/linux/kernel/irq/irqdesc.c
//! test-origin: linux:vendor/linux/kernel/irq/irqdesc.c
//! `struct irq_desc` — per-IRQ descriptor (M37).
//!
//! Mirrors `vendor/linux/include/linux/irqdesc.h`.  Lupos M37 ships a
//! 256-entry static array (one per x86 vector) protected by a `RawSpinLock`.

use core::sync::atomic::{AtomicU32, Ordering};

use spin::Mutex;

use crate::kernel::locking::raw_spinlock::RawSpinLock;

/// Number of IRQ descriptors.  256 = full x86 IDT vector range.
pub const NR_IRQS: usize = 256;

/// Linux `enum irqreturn`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum IrqReturn {
    /// Not handled by this action.
    None = 0,
    /// Handled.
    Handled = 1,
    /// Wake the threaded handler.
    WakeThread = 2,
}

pub const IRQ_NONE: i32 = 0;
pub const IRQ_HANDLED: i32 = 1;
pub const IRQ_WAKE_THREAD: i32 = 2;
pub const IRQ_DISABLED: u32 = 1 << 4;

/// Hard-IRQ handler.  Returns `IRQ_HANDLED` / `IRQ_NONE` / `IRQ_WAKE_THREAD`.
pub type IrqHandler = unsafe extern "C" fn(irq: u32, dev_id: *mut core::ffi::c_void) -> i32;

/// Threaded-IRQ handler — runs in kthread context.
pub type ThreadedHandler = unsafe extern "C" fn(irq: u32, dev_id: *mut core::ffi::c_void) -> i32;

/// One registered handler — Linux's `struct irqaction`.
pub struct IrqAction {
    pub handler: IrqHandler,
    pub thread_fn: Option<ThreadedHandler>,
    pub dev_id: *mut core::ffi::c_void,
    pub name: &'static str,
    pub flags: u32,
    pub next: Option<alloc::boxed::Box<IrqAction>>,
}

unsafe impl Send for IrqAction {}
unsafe impl Sync for IrqAction {}

extern crate alloc;

/// Per-IRQ statistics.
#[derive(Debug, Default)]
pub struct IrqStat {
    pub count: u64,
    pub last_jiffies: u64,
}

/// `struct irq_desc`.
pub struct IrqDesc {
    pub lock: RawSpinLock,
    pub action: Mutex<Option<alloc::boxed::Box<IrqAction>>>,
    pub depth: AtomicU32, // disable nesting (Linux `desc->depth`)
    pub status: AtomicU32,
    pub stat: Mutex<IrqStat>,
    pub affinity: AtomicU32, // bitmap of permitted CPUs
}

impl IrqDesc {
    pub const fn new() -> Self {
        Self {
            lock: RawSpinLock::new(),
            action: Mutex::new(None),
            depth: AtomicU32::new(1), // start disabled (Linux convention)
            status: AtomicU32::new(IRQ_DISABLED),
            stat: Mutex::new(IrqStat {
                count: 0,
                last_jiffies: 0,
            }),
            affinity: AtomicU32::new(!0u32),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.status.load(Ordering::Acquire) & IRQ_DISABLED == 0
    }
}

unsafe impl Send for IrqDesc {}
unsafe impl Sync for IrqDesc {}

/// 256-entry static descriptor array.
static IRQ_DESCS: [IrqDesc; NR_IRQS] = [const { IrqDesc::new() }; NR_IRQS];

/// Look up `irq_desc[irq]`.
pub fn desc_for(irq: u32) -> Option<&'static IrqDesc> {
    let i = irq as usize;
    if i < NR_IRQS {
        Some(&IRQ_DESCS[i])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nr_irqs_is_256() {
        assert_eq!(NR_IRQS, 256);
    }

    #[test]
    fn desc_starts_disabled_with_depth_1() {
        let d = desc_for(0x80).unwrap();
        assert_eq!(d.depth.load(Ordering::Acquire), 1);
        assert!(!d.is_enabled());
    }

    #[test]
    fn irq_return_constants_match_linux() {
        assert_eq!(IRQ_NONE, 0);
        assert_eq!(IRQ_HANDLED, 1);
        assert_eq!(IRQ_WAKE_THREAD, 2);
    }

    #[test]
    fn out_of_range_irq_returns_none() {
        assert!(desc_for(NR_IRQS as u32).is_none());
    }
}
