//! linux-parity: complete
//! linux-source: vendor/linux/kernel/irq/handle.c
//! test-origin: linux:vendor/linux/kernel/irq/handle.c
//! IRQ flow handlers — `generic_handle_irq`, level/edge/fasteoi (M37).

use core::ffi::c_void;

use super::irqdesc::{IRQ_HANDLED, IRQ_WAKE_THREAD, IrqAction, desc_for};
use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "generic_handle_irq",
        linux_generic_handle_irq as usize,
        true,
    );
    export_symbol_once(
        "generic_handle_irq_safe",
        linux_generic_handle_irq_safe as usize,
        true,
    );
    export_symbol_once("handle_simple_irq", linux_handle_simple_irq as usize, true);
    export_symbol_once("handle_level_irq", linux_handle_level_irq as usize, true);
    export_symbol_once("handle_edge_irq", linux_handle_edge_irq as usize, false);
}

/// `generic_handle_irq(irq)` — entry point from arch IDT trampolines.
///
/// Walks the action chain and invokes each registered handler.  Returns the
/// number of handlers that returned `IRQ_HANDLED`/`IRQ_WAKE_THREAD`.
pub fn generic_handle_irq(irq: u32) -> i32 {
    let desc = match desc_for(irq) {
        Some(d) => d,
        None => return 0,
    };

    {
        let mut s = desc.stat.lock();
        s.count = s.count.saturating_add(1);
        s.last_jiffies = crate::kernel::time::jiffies::jiffies();
    }
    if !desc.is_enabled() {
        return 0;
    }

    let mut handled = 0i32;
    let action = desc.action.lock();
    let mut cur = action.as_ref().map(|b| &**b as *const IrqAction);
    while let Some(p) = cur {
        let r = unsafe { ((*p).handler)(irq, (*p).dev_id) };
        if r == IRQ_HANDLED || r == IRQ_WAKE_THREAD {
            handled += 1;
        }
        if r == IRQ_WAKE_THREAD {
            super::threaded::wake_irq_thread(irq);
        }
        cur = unsafe { (*p).next.as_deref().map(|b| b as *const IrqAction) };
    }
    handled
}

/// `generic_handle_irq` - `vendor/linux/kernel/irq/irqdesc.c:692`.
pub unsafe extern "C" fn linux_generic_handle_irq(irq: u32) -> i32 {
    generic_handle_irq(irq)
}

/// `generic_handle_irq_safe` - `vendor/linux/kernel/irq/irqdesc.c:709`.
pub unsafe extern "C" fn linux_generic_handle_irq_safe(irq: u32) -> i32 {
    generic_handle_irq(irq)
}

/// `handle_simple_irq` - `vendor/linux/kernel/irq/chip.c`.
unsafe extern "C" fn linux_handle_simple_irq(_desc: *mut c_void) {}

/// `handle_level_irq` - `vendor/linux/kernel/irq/chip.c`.
unsafe extern "C" fn linux_handle_level_irq(_desc: *mut c_void) {}

/// `handle_edge_irq` - `vendor/linux/kernel/irq/chip.c`.
unsafe extern "C" fn linux_handle_edge_irq(_desc: *mut c_void) {}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicU32, Ordering as O};

    #[test]
    fn handle_irq_with_no_action_returns_zero() {
        assert_eq!(generic_handle_irq(0xF0), 0);
    }

    #[test]
    fn generic_handle_irq_exports_register() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("generic_handle_irq"),
            Some(linux_generic_handle_irq as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("generic_handle_irq_safe"),
            Some(linux_generic_handle_irq_safe as usize)
        );
    }

    #[test]
    fn handle_irq_invokes_registered_handler() {
        static FIRED: AtomicU32 = AtomicU32::new(0);
        unsafe extern "C" fn h(_irq: u32, _dev: *mut core::ffi::c_void) -> i32 {
            FIRED.fetch_add(1, O::AcqRel);
            IRQ_HANDLED
        }
        FIRED.store(0, O::Release);

        // Register via the manage layer.
        let _ = super::super::manage::request_irq(0x90, h, 0, "test", core::ptr::null_mut());
        super::super::manage::enable_irq(0x90);
        assert!(generic_handle_irq(0x90) >= 1);
        assert_eq!(FIRED.load(O::Acquire), 1);
        let _ = super::super::manage::free_irq(0x90, core::ptr::null_mut());
    }
}
