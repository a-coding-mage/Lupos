//! linux-parity: partial
//! linux-source: vendor/linux/kernel/irq/irqdesc.c
//! test-origin: linux:vendor/linux/kernel/irq/irqdesc.c
//! `struct irq_desc` — per-IRQ descriptor (M37).
//!
//! Mirrors `vendor/linux/include/linux/irqdesc.h`.  Lupos M37 ships a
//! 256-entry static array (one per x86 vector) protected by a `RawSpinLock`.

use core::ffi::c_void;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

use spin::Mutex;

use crate::kernel::locking::raw_spinlock::RawSpinLock;
use crate::kernel::module::{export_symbol, find_symbol};

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
const EEXIST: i32 = 17;
const EINVAL: i32 = 22;
const ENOMEM: i32 = 12;

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
    /// Linux `irq_desc::affinity_hint`.  The mask is owned by the caller, as
    /// it is in Linux; `__irq_apply_affinity_hint()` only publishes the
    /// pointer while holding the descriptor lock.
    pub affinity_hint: AtomicUsize,
    pub chip: AtomicUsize,
    pub chip_data: AtomicUsize,
    pub flow_handler: AtomicUsize,
    pub flow_handler_name: AtomicUsize,
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
            affinity_hint: AtomicUsize::new(0),
            chip: AtomicUsize::new(0),
            chip_data: AtomicUsize::new(0),
            flow_handler: AtomicUsize::new(0),
            flow_handler_name: AtomicUsize::new(0),
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
static IRQ_ALLOCATED: [AtomicBool; NR_IRQS] = [const { AtomicBool::new(false) }; NR_IRQS];

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("irq_free_descs", linux_irq_free_descs as usize, true);
    export_symbol_once("__irq_alloc_descs", linux___irq_alloc_descs as usize, true);
}

/// Look up `irq_desc[irq]`.
pub fn desc_for(irq: u32) -> Option<&'static IrqDesc> {
    let i = irq as usize;
    if i < NR_IRQS {
        Some(&IRQ_DESCS[i])
    } else {
        None
    }
}

fn reset_desc(desc: &IrqDesc) {
    *desc.action.lock() = None;
    desc.depth.store(1, Ordering::Release);
    desc.status.store(IRQ_DISABLED, Ordering::Release);
    desc.affinity.store(!0u32, Ordering::Release);
    desc.affinity_hint.store(0, Ordering::Release);
    desc.chip.store(0, Ordering::Release);
    desc.chip_data.store(0, Ordering::Release);
    desc.flow_handler.store(0, Ordering::Release);
    desc.flow_handler_name.store(0, Ordering::Release);
    *desc.stat.lock() = IrqStat {
        count: 0,
        last_jiffies: 0,
    };
}

fn desc_range_free(start: usize, count: usize) -> bool {
    let Some(end) = start.checked_add(count) else {
        return false;
    };
    if start >= NR_IRQS || end > NR_IRQS {
        return false;
    }

    (start..end).all(|irq| {
        !IRQ_ALLOCATED[irq].load(Ordering::Acquire) && IRQ_DESCS[irq].action.lock().is_none()
    })
}

fn mark_desc_range_allocated(start: usize, count: usize) {
    for irq in start..start + count {
        reset_desc(&IRQ_DESCS[irq]);
        IRQ_ALLOCATED[irq].store(true, Ordering::Release);
    }
}

fn find_free_desc_range(from: usize, count: usize) -> Option<usize> {
    if count == 0 || count > NR_IRQS {
        return None;
    }
    let mut start = from.min(NR_IRQS);
    while start + count <= NR_IRQS {
        if desc_range_free(start, count) {
            return Some(start);
        }
        start += 1;
    }
    None
}

/// `irq_free_descs` - `vendor/linux/kernel/irq/irqdesc.c:865`.
#[unsafe(export_name = "irq_free_descs")]
pub unsafe extern "C" fn linux_irq_free_descs(from: u32, cnt: u32) {
    let start = from as usize;
    let count = cnt as usize;
    let Some(end) = start.checked_add(count) else {
        return;
    };
    if start >= NR_IRQS || end > NR_IRQS {
        return;
    }

    for irq in start..end {
        reset_desc(&IRQ_DESCS[irq]);
        IRQ_ALLOCATED[irq].store(false, Ordering::Release);
    }
}

/// `__irq_alloc_descs` - `vendor/linux/kernel/irq/irqdesc.c:891`.
#[unsafe(export_name = "__irq_alloc_descs")]
pub unsafe extern "C" fn linux___irq_alloc_descs(
    irq: i32,
    from: u32,
    cnt: u32,
    _node: i32,
    _owner: *mut c_void,
    _affinity: *const c_void,
) -> i32 {
    if cnt == 0 {
        return -EINVAL;
    }

    let count = cnt as usize;
    if irq >= 0 {
        if from > irq as u32 {
            return -EINVAL;
        }
        let start = irq as usize;
        if start.checked_add(count).is_none_or(|end| end > NR_IRQS) {
            return -ENOMEM;
        }
        if !desc_range_free(start, count) {
            return -EEXIST;
        }
        mark_desc_range_allocated(start, count);
        return irq;
    }

    let Some(start) = find_free_desc_range((from as usize).max(1), count) else {
        return -ENOMEM;
    };
    mark_desc_range_allocated(start, count);
    start as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe extern "C" fn dummy_handler(_irq: u32, _dev_id: *mut core::ffi::c_void) -> i32 {
        IRQ_HANDLED
    }

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

    #[test]
    fn irqdesc_exports_include_irq_free_descs() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/irq/irqdesc.c"
        ));
        assert!(source.contains("EXPORT_SYMBOL_GPL(irq_free_descs);"));

        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("irq_free_descs"),
            Some(linux_irq_free_descs as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("__irq_alloc_descs"),
            Some(linux___irq_alloc_descs as usize)
        );
    }

    #[test]
    fn irq_free_descs_resets_static_descriptor_state() {
        let irq = 0x7du32;
        let desc = desc_for(irq).unwrap();
        *desc.action.lock() = Some(alloc::boxed::Box::new(IrqAction {
            handler: dummy_handler,
            thread_fn: None,
            dev_id: 1 as *mut _,
            name: "test",
            flags: 0,
            next: None,
        }));
        desc.depth.store(0, Ordering::Release);
        desc.status.store(0, Ordering::Release);
        desc.affinity.store(1, Ordering::Release);
        desc.affinity_hint.store(0x1234, Ordering::Release);
        desc.chip_data.store(0x5678, Ordering::Release);

        unsafe { linux_irq_free_descs(irq, 1) };

        assert!(desc.action.lock().is_none());
        assert_eq!(desc.depth.load(Ordering::Acquire), 1);
        assert_eq!(desc.status.load(Ordering::Acquire), IRQ_DISABLED);
        assert_eq!(desc.affinity.load(Ordering::Acquire), !0u32);
        assert_eq!(desc.affinity_hint.load(Ordering::Acquire), 0);
        assert_eq!(desc.chip_data.load(Ordering::Acquire), 0);
    }

    #[test]
    fn irq_alloc_descs_reserves_exact_and_dynamic_ranges() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/irq/irqdesc.c"
        ));
        assert!(source.contains("EXPORT_SYMBOL_GPL(__irq_alloc_descs);"));

        let exact = unsafe {
            linux___irq_alloc_descs(0xa0, 0xa0, 2, -1, core::ptr::null_mut(), core::ptr::null())
        };
        assert_eq!(exact, 0xa0);
        assert_eq!(
            unsafe {
                linux___irq_alloc_descs(0xa0, 0xa0, 1, -1, core::ptr::null_mut(), core::ptr::null())
            },
            -EEXIST
        );
        unsafe { linux_irq_free_descs(0xa0, 2) };

        let dynamic = unsafe {
            linux___irq_alloc_descs(-1, 0xa0, 2, -1, core::ptr::null_mut(), core::ptr::null())
        };
        assert!(dynamic >= 0xa0);
        unsafe { linux_irq_free_descs(dynamic as u32, 2) };
    }
}
