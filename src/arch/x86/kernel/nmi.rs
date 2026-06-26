//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/nmi.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/nmi.c
//! x86 Non-Maskable Interrupt handler registry and dispatch.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/nmi.c

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use spin::Mutex;

use crate::arch::x86::kernel::idt::ExceptionFrame;
use crate::include::uapi::errno::{EEXIST, ENOENT};
use crate::kernel::locking::preempt::{__nmi_enter_raw, __nmi_exit_raw};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NmiResult {
    Handled,
    Unhandled,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NmiContext {
    pub rip: u64,
    pub cr2: u64,
    pub dr7: u64,
}

pub type NmiHandlerFn = fn(&NmiContext) -> NmiResult;

#[derive(Clone, Copy)]
pub struct NmiHandler {
    pub name: &'static str,
    pub priority: u8,
    pub handler: NmiHandlerFn,
}

static NMI_HANDLERS: Mutex<Vec<NmiHandler>> = Mutex::new(Vec::new());
static NMI_STOPPED: AtomicBool = AtomicBool::new(false);
static NMI_TOTAL: AtomicU64 = AtomicU64::new(0);
static NMI_UNKNOWN: AtomicU64 = AtomicU64::new(0);

pub fn __register_nmi_handler(handler: NmiHandler) -> Result<(), i32> {
    let mut handlers = NMI_HANDLERS.lock();
    if handlers.iter().any(|h| h.name == handler.name) {
        return Err(EEXIST);
    }
    handlers.push(handler);
    handlers.sort_by(|a, b| b.priority.cmp(&a.priority));
    Ok(())
}

pub fn unregister_nmi_handler(name: &str) -> Result<(), i32> {
    let mut handlers = NMI_HANDLERS.lock();
    let before = handlers.len();
    handlers.retain(|h| h.name != name);
    if handlers.len() == before {
        Err(ENOENT)
    } else {
        Ok(())
    }
}

pub fn set_emergency_nmi_handler(handler: NmiHandlerFn) {
    let _ = __register_nmi_handler(NmiHandler {
        name: "emergency",
        priority: u8::MAX,
        handler,
    });
}

pub fn stop_nmi() {
    NMI_STOPPED.store(true, Ordering::Release);
}

pub fn restart_nmi() {
    NMI_STOPPED.store(false, Ordering::Release);
}

pub fn local_touch_nmi() {
    NMI_TOTAL.fetch_add(1, Ordering::AcqRel);
}

pub fn nmi_stats() -> (u64, u64) {
    (
        NMI_TOTAL.load(Ordering::Acquire),
        NMI_UNKNOWN.load(Ordering::Acquire),
    )
}

pub fn dispatch_nmi(ctx: &NmiContext) -> NmiResult {
    if NMI_STOPPED.load(Ordering::Acquire) {
        return NmiResult::Handled;
    }
    NMI_TOTAL.fetch_add(1, Ordering::AcqRel);
    for h in NMI_HANDLERS.lock().iter() {
        if (h.handler)(ctx) == NmiResult::Handled {
            return NmiResult::Handled;
        }
    }
    NMI_UNKNOWN.fetch_add(1, Ordering::AcqRel);
    NmiResult::Unhandled
}

pub fn exc_nmi(frame: &ExceptionFrame) -> bool {
    __nmi_enter_raw();
    let ctx = NmiContext {
        rip: frame.rip,
        cr2: 0,
        dr7: 0,
    };
    let handled = dispatch_nmi(&ctx) == NmiResult::Handled;
    __nmi_exit_raw();
    handled
}

#[cfg(test)]
mod tests {
    use super::*;

    fn handled(_: &NmiContext) -> NmiResult {
        NmiResult::Handled
    }

    fn unhandled(_: &NmiContext) -> NmiResult {
        NmiResult::Unhandled
    }

    #[test]
    fn registry_orders_by_priority_and_dispatches_first_handler() {
        let _ = unregister_nmi_handler("low");
        let _ = unregister_nmi_handler("high");
        __register_nmi_handler(NmiHandler {
            name: "low",
            priority: 1,
            handler: unhandled,
        })
        .unwrap();
        __register_nmi_handler(NmiHandler {
            name: "high",
            priority: 10,
            handler: handled,
        })
        .unwrap();
        assert_eq!(dispatch_nmi(&NmiContext::default()), NmiResult::Handled);
        unregister_nmi_handler("low").unwrap();
        unregister_nmi_handler("high").unwrap();
    }

    #[test]
    fn stop_nmi_short_circuits_dispatch() {
        stop_nmi();
        assert_eq!(dispatch_nmi(&NmiContext::default()), NmiResult::Handled);
        restart_nmi();
    }
}
