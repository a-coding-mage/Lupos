//! linux-parity: partial
//! linux-source: vendor/linux/kernel/irq/manage.c
//! test-origin: linux:vendor/linux/kernel/irq/manage.c
//! IRQ management — `request_irq` / `free_irq` / `enable_irq` / `disable_irq`
//! / `irq_set_affinity` (M37).

extern crate alloc;

use core::ffi::{c_char, c_void};
use core::sync::atomic::Ordering;

use super::irqdesc::{IRQ_DISABLED, IrqAction, IrqHandler, ThreadedHandler, desc_for};
use crate::arch::x86::kernel::cpu::common::LinuxCpuMask;
use crate::kernel::module::{export_symbol, find_symbol};

// ── Linux `IRQF_*` flags ────────────────────────────────────────────────────

pub const IRQF_SHARED: u32 = 0x0080;
pub const IRQF_TRIGGER_RISING: u32 = 0x0001;
pub const IRQF_TRIGGER_FALLING: u32 = 0x0002;
pub const IRQF_TRIGGER_HIGH: u32 = 0x0004;
pub const IRQF_TRIGGER_LOW: u32 = 0x0008;
pub const IRQF_ONESHOT: u32 = 0x2000;

// ── errno values ────────────────────────────────────────────────────────────

pub const EINVAL: i32 = 22;
pub const EBUSY: i32 = 16;
pub const ENXIO: i32 = 6;

fn export_symbol_once(name: &str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "request_threaded_irq",
        linux_request_threaded_irq as usize,
        false,
    );
    export_symbol_once("free_irq", linux_free_irq as usize, false);
    export_symbol_once("synchronize_irq", linux_synchronize_irq as usize, false);
    export_symbol_once(
        "__irq_apply_affinity_hint",
        linux_irq_apply_affinity_hint as usize,
        true,
    );
}

/// `request_irq(irq, handler, flags, name, dev_id)`.
pub fn request_irq(
    irq: u32,
    handler: IrqHandler,
    flags: u32,
    name: &'static str,
    dev_id: *mut core::ffi::c_void,
) -> Result<(), i32> {
    let desc = desc_for(irq).ok_or(ENXIO)?;
    let mut slot = desc.action.lock();
    if slot.is_some() && flags & IRQF_SHARED == 0 {
        return Err(EBUSY);
    }
    let action = alloc::boxed::Box::new(IrqAction {
        handler,
        thread_fn: None,
        dev_id,
        name,
        flags,
        next: slot.take(),
    });
    *slot = Some(action);
    drop(slot);
    startup_irq(irq);
    Ok(())
}

/// `request_threaded_irq(irq, handler, thread_fn, flags, name, dev_id)`.
pub fn request_threaded_irq(
    irq: u32,
    handler: IrqHandler,
    thread_fn: ThreadedHandler,
    flags: u32,
    name: &'static str,
    dev_id: *mut core::ffi::c_void,
) -> Result<(), i32> {
    let desc = desc_for(irq).ok_or(ENXIO)?;
    let mut slot = desc.action.lock();
    if slot.is_some() && flags & IRQF_SHARED == 0 {
        return Err(EBUSY);
    }
    let action = alloc::boxed::Box::new(IrqAction {
        handler,
        thread_fn: Some(thread_fn),
        dev_id,
        name,
        flags,
        next: slot.take(),
    });
    *slot = Some(action);
    drop(slot);
    startup_irq(irq);
    Ok(())
}

/// `free_irq(irq, dev_id)` — remove the action whose `dev_id` matches.
pub fn free_irq(irq: u32, dev_id: *mut core::ffi::c_void) -> Result<(), i32> {
    let desc = desc_for(irq).ok_or(ENXIO)?;
    let mut slot = desc.action.lock();
    // Walk the action chain and unlink the first match.
    let mut cursor: &mut Option<alloc::boxed::Box<IrqAction>> = &mut *slot;
    while let Some(b) = cursor {
        if b.dev_id == dev_id {
            let removed = cursor.take().unwrap();
            *cursor = removed.next; // move successor into our slot
            return Ok(());
        }
        cursor = &mut cursor.as_mut().unwrap().next;
    }
    Err(EINVAL)
}

/// `enable_irq(irq)` — decrement disable depth; if it reaches zero, clear
/// `IRQ_DISABLED`.
pub fn enable_irq(irq: u32) {
    if let Some(desc) = desc_for(irq) {
        loop {
            let depth = desc.depth.load(Ordering::Acquire);
            if depth == 0 {
                desc.status.fetch_and(!IRQ_DISABLED, Ordering::AcqRel);
                return;
            }
            if desc
                .depth
                .compare_exchange(depth, depth - 1, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                if depth == 1 {
                    desc.status.fetch_and(!IRQ_DISABLED, Ordering::AcqRel);
                }
                return;
            }
        }
    }
}

/// `disable_irq(irq)` — increment depth; set `IRQ_DISABLED`.
pub fn disable_irq(irq: u32) {
    if let Some(desc) = desc_for(irq) {
        desc.depth.fetch_add(1, Ordering::AcqRel);
        desc.status.fetch_or(IRQ_DISABLED, Ordering::AcqRel);
    }
}

/// `irq_set_affinity(irq, mask)`.
pub fn irq_set_affinity(irq: u32, mask: u32) -> Result<(), i32> {
    let desc = desc_for(irq).ok_or(ENXIO)?;
    if u64::from(mask) & super::cpuhotplug::irq_online_cpu_mask() == 0 {
        return Err(EINVAL);
    }
    desc.affinity.store(mask, Ordering::Release);
    Ok(())
}

/// `__irq_apply_affinity_hint` - `vendor/linux/kernel/irq/manage.c:504`.
///
/// Linux retains the caller-owned mask pointer in `irq_desc::affinity_hint`.
/// When requested it then attempts to apply the same mask, but deliberately
/// does not replace the successful hint-update return value with an affinity
/// programming error.
#[unsafe(export_name = "__irq_apply_affinity_hint")]
pub unsafe extern "C" fn linux_irq_apply_affinity_hint(
    irq: u32,
    mask: *const LinuxCpuMask,
    set_affinity: bool,
) -> i32 {
    let Some(desc) = desc_for(irq) else {
        return -EINVAL;
    };

    desc.affinity_hint.store(mask as usize, Ordering::Release);
    if !mask.is_null() && set_affinity {
        // The staged x86_64 vendor configuration has NR_CPUS=64, hence a
        // Linux cpumask occupies exactly one machine word.  Lupos's IRQ
        // descriptor currently tracks its supported CPU slots in its u32
        // affinity field; CPUs outside that field cannot be online.
        let requested = unsafe { (*mask).bits[0] } as u32;
        let _ = irq_set_affinity(irq, requested);
    }
    0
}

fn startup_irq(irq: u32) {
    enable_irq(irq);
    #[cfg(not(test))]
    if irq < 16 {
        unsafe {
            crate::arch::x86::kernel::apic::enable_lint0_extint();
            crate::arch::x86::kernel::apic_io_apic::route_pci_intx_for_legacy_irq(irq as u8);
            crate::arch::x86::kernel::pic::unmask_irq(irq as u8);
        }
    }
}

unsafe extern "C" fn irq_default_primary_handler(_irq: u32, _dev_id: *mut c_void) -> i32 {
    super::irqdesc::IRQ_WAKE_THREAD
}

/// `request_threaded_irq` - `vendor/linux/kernel/irq/manage.c:2115`.
#[unsafe(export_name = "request_threaded_irq")]
pub unsafe extern "C" fn linux_request_threaded_irq(
    irq: u32,
    handler: Option<IrqHandler>,
    thread_fn: Option<ThreadedHandler>,
    irqflags: usize,
    _devname: *const c_char,
    dev_id: *mut c_void,
) -> i32 {
    if handler.is_none() && thread_fn.is_none() {
        crate::log_warn!(
            "irq",
            "request_threaded_irq: irq {} has no handler or thread_fn",
            irq
        );
        return -EINVAL;
    }
    if (irqflags as u32 & IRQF_SHARED) != 0 && dev_id.is_null() {
        crate::log_warn!(
            "irq",
            "request_threaded_irq: shared irq {} missing dev_id flags={:#x}",
            irq,
            irqflags
        );
        return -EINVAL;
    }

    let Some(desc) = desc_for(irq) else {
        crate::log_warn!("irq", "request_threaded_irq: irq {} has no descriptor", irq);
        return -ENXIO;
    };
    let mut slot = desc.action.lock();
    if slot.is_some() && (irqflags as u32 & IRQF_SHARED) == 0 {
        crate::log_warn!(
            "irq",
            "request_threaded_irq: irq {} busy flags={:#x}",
            irq,
            irqflags
        );
        return -EBUSY;
    }
    let action = alloc::boxed::Box::new(IrqAction {
        handler: handler.unwrap_or(irq_default_primary_handler),
        thread_fn,
        dev_id,
        name: "module-irq",
        flags: irqflags as u32,
        next: slot.take(),
    });
    *slot = Some(action);
    drop(slot);
    startup_irq(irq);
    0
}

/// `free_irq` - `vendor/linux/kernel/irq/manage.c:2004`.
#[unsafe(export_name = "free_irq")]
pub unsafe extern "C" fn linux_free_irq(irq: u32, dev_id: *mut c_void) -> *const c_void {
    match free_irq(irq, dev_id) {
        Ok(()) => core::ptr::null(),
        Err(_) => core::ptr::null(),
    }
}

/// `synchronize_irq` - `vendor/linux/kernel/irq/manage.c:145`.
#[unsafe(export_name = "synchronize_irq")]
pub unsafe extern "C" fn linux_synchronize_irq(_irq: u32) {}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe extern "C" fn dummy_handler(_irq: u32, _dev: *mut core::ffi::c_void) -> i32 {
        super::super::irqdesc::IRQ_HANDLED
    }

    #[test]
    fn request_then_free_irq_round_trip() {
        request_irq(0x70, dummy_handler, 0, "t", core::ptr::null_mut()).unwrap();
        free_irq(0x70, core::ptr::null_mut()).unwrap();
    }

    #[test]
    fn duplicate_request_without_shared_returns_ebusy() {
        request_irq(0x71, dummy_handler, 0, "t1", 1 as *mut _).unwrap();
        let r = request_irq(0x71, dummy_handler, 0, "t2", 2 as *mut _);
        assert_eq!(r, Err(EBUSY));
        let _ = free_irq(0x71, 1 as *mut _);
    }

    #[test]
    fn enable_disable_round_trip_clears_status() {
        let irq = 0x72u32;
        request_irq(irq, dummy_handler, 0, "t", core::ptr::null_mut()).unwrap();
        enable_irq(irq);
        let desc = desc_for(irq).unwrap();
        assert!(desc.is_enabled());
        disable_irq(irq);
        assert!(!desc.is_enabled());
        let _ = free_irq(irq, core::ptr::null_mut());
    }

    #[test]
    fn irq_set_affinity_updates_field() {
        let irq = 0x73u32;
        request_irq(irq, dummy_handler, 0, "t", core::ptr::null_mut()).unwrap();
        irq_set_affinity(irq, 0xF).unwrap();
        let desc = desc_for(irq).unwrap();
        assert_eq!(desc.affinity.load(Ordering::Acquire), 0xF);
        let _ = free_irq(irq, core::ptr::null_mut());
    }

    #[test]
    fn unknown_irq_returns_enxio() {
        assert_eq!(
            request_irq(
                super::super::irqdesc::NR_IRQS as u32,
                dummy_handler,
                0,
                "x",
                core::ptr::null_mut()
            ),
            Err(ENXIO),
        );
    }

    #[test]
    fn irqf_constants_match_linux() {
        assert_eq!(IRQF_SHARED, 0x80);
        assert_eq!(IRQF_TRIGGER_RISING, 1);
    }

    #[test]
    fn linux_irq_module_exports_register() {
        register_module_exports();

        assert_eq!(
            crate::kernel::module::find_symbol("request_threaded_irq"),
            Some(linux_request_threaded_irq as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("free_irq"),
            Some(linux_free_irq as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("synchronize_irq"),
            Some(linux_synchronize_irq as usize)
        );
    }

    #[test]
    fn linux_request_threaded_irq_wrapper_matches_basic_validation() {
        unsafe {
            assert_eq!(
                linux_request_threaded_irq(0x74, None, None, 0, core::ptr::null(), 1 as *mut _),
                -EINVAL
            );
            assert_eq!(
                linux_request_threaded_irq(
                    0x75,
                    Some(dummy_handler),
                    None,
                    IRQF_SHARED as usize,
                    core::ptr::null(),
                    core::ptr::null_mut()
                ),
                -EINVAL
            );
            assert_eq!(
                linux_request_threaded_irq(
                    0x76,
                    Some(dummy_handler),
                    None,
                    0,
                    core::ptr::null(),
                    1 as *mut _
                ),
                0
            );
            let _ = linux_free_irq(0x76, 1 as *mut _);
            linux_synchronize_irq(0x76);
        }
    }
}
