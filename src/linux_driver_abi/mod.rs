//! linux-parity: partial
//! linux-source: vendor/linux/drivers
//! Lupos Linux driver ABI glue.
//!
//! This module owns the kernel-facing ABI that Linux-built driver artifacts
//! call into: device model registration, bus/class plumbing, sysfs/devtmpfs
//! helpers, and boot-console integration. Device-driver implementations must
//! come from `vendor/linux` builds, not Rust-written local payloads.

extern crate alloc;

use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;

pub mod base; // M54
pub mod block; // M44/M57
pub mod gpu; // M57
pub mod hid; // M58
pub mod input; // M58
pub mod iommu; // M55
pub mod pci; // M55
pub mod platform;
pub mod storage_core;
pub mod tty; // M57
pub mod usb; // M58
pub mod video; // graphics/Wayland prereq
#[cfg(any(test, CONFIG_VIRTIO = "y", CONFIG_VIRTIO = "m"))]
pub mod virtio; // M57

pub(crate) type DriverAbiPollFn = fn() -> usize;

#[derive(Clone, Copy)]
struct DriverAbiPoller {
    name: &'static str,
    poll: DriverAbiPollFn,
}

lazy_static! {
    static ref DRIVER_ABI_POLLERS: Mutex<Vec<DriverAbiPoller>> = Mutex::new(Vec::new());
}

/// Register a core ABI event poller used when Lupos lacks a native IRQ
/// delivery path for a Linux-built module's bus/core callback.
///
/// This is intentionally generic: block, PCI, and boot code must not call into
/// a local Rust virtio/blk driver path. Actual drivers still arrive as
/// `vendor/linux` `.ko` payloads; pollers only surface core completion events
/// to those Linux callbacks.
pub(crate) fn register_driver_abi_poller(name: &'static str, poll: DriverAbiPollFn) {
    let mut pollers = DRIVER_ABI_POLLERS.lock();
    if pollers.iter().any(|registered| registered.name == name) {
        return;
    }
    pollers.push(DriverAbiPoller { name, poll });
}

/// Reentrancy guard for `poll_driver_abi_events`. A poller's completion chain
/// (e.g. the AHCI reaper: `ahci_handle_port_intr` → `ata_qc_complete` → the SCSI
/// completion path) can yield to the scheduler mid-completion. Without this
/// guard, a peer task's block wait loop would then re-enter the same poller —
/// re-running `ahci_handle_port_intr` on the same port — and corrupt libata's
/// per-port command state (`ap->qc_active`/`link->active_tag`/`link->sactive`),
/// the non-deterministic completion leak that left real VirtualBox unable to
/// drain its queue. Serializing the pollers (a waiter that finds it busy returns
/// 0 and retries next iteration) removes the reentrancy without deadlock: the
/// holder always finishes and clears it.
static DRIVER_POLL_REENTRANT_GUARD: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);

pub(crate) fn poll_driver_abi_events() -> usize {
    use core::sync::atomic::Ordering;
    if DRIVER_POLL_REENTRANT_GUARD.swap(true, Ordering::AcqRel) {
        return 0;
    }
    let pollers = DRIVER_ABI_POLLERS.lock().clone();
    let handled = pollers.iter().map(|poller| (poller.poll)()).sum();
    DRIVER_POLL_REENTRANT_GUARD.store(false, Ordering::Release);
    handled
}

/// Register Linux driver ABI symbols before vendor-built `.ko` modules load.
pub fn register_module_exports() {
    crate::lib::register_module_exports();
    crate::kernel::params::register_module_exports();
    crate::arch::x86::kernel::head64::register_module_exports();
    crate::arch::x86::kernel::cpu::common::register_module_exports();
    crate::arch::x86::entry::thunk::register_module_exports();
    crate::mm::ioremap::register_module_exports();
    crate::mm::page_alloc::register_module_exports();
    crate::mm::slab::register_module_exports();
    crate::mm::vmalloc::register_module_exports();
    crate::kernel::dma::register_module_exports();
    crate::kernel::cpuhotplug::register_module_exports();
    crate::kernel::irq::register_module_exports();
    crate::kernel::sched::register_module_exports();
    crate::kernel::time::jiffies::register_module_exports();
    crate::kernel::time::sleep_timeout::register_module_exports();
    crate::kernel::workqueue::register_module_exports();
    crate::kernel::locking::preempt::register_module_exports();
    crate::kernel::locking::mutex::register_module_exports();
    crate::net::core::page_pool::register_module_exports();
    crate::net::module_abi::register_module_exports();
    base::register_module_exports();
    pci::register_module_exports();
    block::register_module_exports();
    storage_core::register_module_exports();
    #[cfg(any(test, CONFIG_VIRTIO = "y", CONFIG_VIRTIO = "m"))]
    virtio::register_module_exports();
}
