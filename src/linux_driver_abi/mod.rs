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
pub mod hwmon;
pub mod i2c;
pub mod input; // M58
pub mod iommu; // M55
pub mod pci; // M55
pub mod platform;
pub mod pnp;
pub mod storage_core;
pub mod thermal;
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

/// Surface vendor-driver completions at a syscall wait boundary, then run any
/// softirqs those callbacks scheduled.  This is task context: unlike hard-IRQ
/// exit, callers hold no socket, epoll, or console locks while the vendor
/// callback and NET_RX action run.
pub(crate) fn poll_driver_abi_events_for_wait() -> usize {
    #[cfg(not(test))]
    {
        let handled = poll_driver_abi_events();
        crate::kernel::workqueue::drain_system_workqueues();
        crate::kernel::softirq::do_softirq();
        crate::kernel::workqueue::drain_system_workqueues();
        handled
    }
    #[cfg(test)]
    {
        0
    }
}

/// Register Linux driver ABI symbols before vendor-built `.ko` modules load.
pub fn register_module_exports() {
    crate::lib::register_module_exports();
    crate::kernel::module::register_module_exports();
    crate::kernel::params::register_module_exports();
    crate::kernel::sysctl_abi::register_module_exports();
    crate::kernel::taint::register_module_exports();
    crate::kernel::events::register_module_exports();
    crate::kernel::printk::register_module_exports();
    crate::kernel::relay::register_module_exports();
    crate::arch::x86::kernel::head64::register_module_exports();
    crate::arch::x86::kernel::acpi::register_module_exports();
    crate::arch::x86::kernel::cpu::common::register_module_exports();
    crate::arch::x86::kernel::dmi::register_module_exports();
    crate::arch::x86::kernel::fpu::register_module_exports();
    crate::arch::x86::kernel::jump_label::register_module_exports();
    crate::arch::x86::kernel::setup::register_module_exports();
    crate::arch::x86::kernel::setup_percpu::register_module_exports();
    crate::arch::x86::kernel::msr::register_module_exports();
    crate::arch::x86::kernel::static_call::register_module_exports();
    crate::arch::x86::kernel::tsc::register_module_exports();
    crate::arch::x86::entry::thunk::register_module_exports();
    crate::arch::x86::lib::cache_smp::register_module_exports();
    crate::arch::x86::lib::page::register_module_exports();
    crate::arch::x86::video::register_module_exports();
    crate::arch::x86::mm::init::register_module_exports();
    crate::arch::x86::mm::pgprot::register_module_exports();
    crate::arch::x86::mm::pat::register_module_exports();
    crate::fs::char_dev::register_module_exports();
    crate::fs::fcntl::register_module_exports();
    crate::fs::ioctl::register_module_exports();
    crate::fs::procfs_abi::register_module_exports();
    crate::fs::anon_inode::register_module_exports();
    crate::fs::eventfd::register_module_exports();
    crate::fs::file::register_module_exports();
    crate::fs::libfs::register_module_exports();
    crate::fs::pipe::register_module_exports();
    crate::fs::read_write::register_module_exports();
    crate::fs::super_block::register_module_exports();
    crate::fs::netfs::register_module_exports();
    crate::mm::gup::register_module_exports();
    crate::mm::ioremap::register_module_exports();
    crate::mm::list_lru::register_module_exports();
    crate::mm::mm_init::register_module_exports();
    crate::mm::mmu_notifier::register_module_exports();
    crate::mm::mm_public::register_module_exports();
    crate::mm::mmap::register_module_exports();
    crate::mm::mmap_lock::register_module_exports();
    crate::mm::page_alloc::register_module_exports();
    crate::mm::filemap::register_module_exports();
    crate::mm::shmem::register_module_exports();
    crate::mm::slab::register_module_exports();
    crate::mm::swap::register_module_exports();
    crate::mm::util::register_module_exports();
    crate::mm::vmstat::register_module_exports();
    crate::mm::vmalloc::register_module_exports();
    crate::arch::x86::kernel::uaccess::register_module_exports();
    crate::kernel::capability::register_module_exports();
    crate::kernel::utsname::register_module_exports();
    crate::kernel::pid::register_module_exports();
    crate::kernel::signal::register_module_exports();
    crate::kernel::dma::register_module_exports();
    crate::kernel::cpuhotplug::register_module_exports();
    crate::kernel::up::register_module_exports();
    crate::kernel::softirq::register_module_exports();
    crate::kernel::irq::register_module_exports();
    crate::kernel::irq_work::register_module_exports();
    crate::kernel::notifier::register_module_exports();
    crate::kernel::rcu::register_module_exports();
    crate::kernel::sched::register_module_exports();
    crate::kernel::stop_machine::register_module_exports();
    crate::kernel::time::clocksource::register_module_exports();
    crate::kernel::time::jiffies::register_module_exports();
    crate::kernel::time::time::register_module_exports();
    crate::kernel::time::timeconv::register_module_exports();
    crate::kernel::time::timecounter::register_module_exports();
    crate::kernel::time::timekeeping::register_module_exports();
    crate::kernel::time::hrtimer::register_module_exports();
    crate::kernel::time::timer::register_module_exports();
    crate::kernel::time::sleep_timeout::register_module_exports();
    crate::kernel::power::register_module_exports();
    crate::io_uring::register_module_exports();
    crate::kernel::workqueue::register_module_exports();
    crate::kernel::locking::preempt::register_module_exports();
    crate::kernel::locking::raw_spinlock::register_module_exports();
    crate::kernel::locking::qrwlock::register_module_exports();
    crate::kernel::locking::mutex::register_module_exports();
    crate::kernel::locking::rtmutex_api::register_module_exports();
    crate::kernel::locking::rwsem::register_module_exports();
    crate::arch::x86::kernel::amd_nb::register_module_exports();
    crate::fs::debugfs::register_module_exports();
    crate::fs::sysfs::register_module_exports();
    crate::fs::seq_file_abi::register_module_exports();
    crate::net::core::page_pool::register_module_exports();
    crate::net::module_abi::register_module_exports();
    base::register_module_exports();
    input::register_module_exports();
    pci::register_module_exports();
    pnp::register_module_exports();
    block::register_module_exports();
    storage_core::register_module_exports();
    thermal::register_module_exports();
    tty::register_module_exports();
    usb::register_module_exports();
    hwmon::register_module_exports();
    i2c::register_module_exports();
    gpu::register_module_exports();
    video::register_module_exports();
    // `gpu::drm::module_abi` remains an implementation work inventory.  Its
    // side-table/fabricated-object shims do not yet preserve the Linux object
    // and lifetime contracts, so registering them would turn honest
    // unresolved-symbol failures into memory corruption during DRM probe.
    #[cfg(any(test, CONFIG_VIRTIO = "y", CONFIG_VIRTIO = "m"))]
    virtio::register_module_exports();
}
