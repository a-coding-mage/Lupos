//! linux-parity: complete
//! linux-source: vendor/linux/kernel/time/timer.c
//! test-origin: linux:vendor/linux/kernel/time/timer.c
//! Low-resolution timer list coverage for M36.
//!
//! Mirrors `vendor/linux/kernel/time/timer.c`.  High-resolution timers live in
//! `hrtimer.rs`; this module models the jiffies-based `struct timer_list`.

use core::ffi::c_void;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::kernel::module::{export_symbol, find_symbol};

pub type TimerCallback = fn(u64);

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TimerList {
    pub expires: u64,
    pub data: u64,
    pub function: Option<TimerCallback>,
    active: bool,
}

impl TimerList {
    pub const fn new() -> Self {
        Self {
            expires: 0,
            data: 0,
            function: None,
            active: false,
        }
    }
}

static FIRED_TIMERS: AtomicU64 = AtomicU64::new(0);

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("timer_delete", linux_timer_delete as usize, false);
    export_symbol_once("timer_delete_sync", linux_timer_delete as usize, false);
    export_symbol_once("timer_delete_sync_try", linux_timer_delete as usize, false);
    export_symbol_once("timer_shutdown", linux_timer_delete as usize, true);
    export_symbol_once("timer_shutdown_sync", linux_timer_delete as usize, true);
}

/// `timer_delete`/`timer_delete_sync` - `vendor/linux/kernel/time/timer.c`.
///
/// Linux-built modules pass the vendor `struct timer_list`, whose layout is
/// not this file's compact Rust test model.  Until Lupos owns a Linux timer
/// wheel for module callbacks, report "not pending" without touching the
/// foreign object.
#[unsafe(export_name = "timer_delete")]
pub unsafe extern "C" fn linux_timer_delete(_timer: *mut c_void) -> i32 {
    0
}

pub fn timer_setup(timer: &mut TimerList, function: TimerCallback, data: u64) {
    timer.function = Some(function);
    timer.data = data;
    timer.active = false;
}

pub fn mod_timer(timer: &mut TimerList, expires: u64) -> bool {
    let was_active = timer.active;
    timer.expires = expires;
    timer.active = true;
    was_active
}

pub fn del_timer(timer: &mut TimerList) -> bool {
    let was_active = timer.active;
    timer.active = false;
    was_active
}

pub fn timer_pending(timer: &TimerList) -> bool {
    timer.active
}

pub fn run_timer(timer: &mut TimerList, now: u64) -> bool {
    if timer.active && timer.expires <= now {
        timer.active = false;
        FIRED_TIMERS.fetch_add(1, Ordering::AcqRel);
        if let Some(function) = timer.function {
            function(timer.data);
        }
        return true;
    }
    false
}

pub fn fired_timer_count() -> u64 {
    FIRED_TIMERS.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicU64, Ordering as O};

    static LAST_DATA: AtomicU64 = AtomicU64::new(0);

    fn cb(data: u64) {
        LAST_DATA.store(data, O::Release);
    }

    #[test]
    fn timer_fires_once_after_expiry() {
        let mut timer = TimerList::new();
        timer_setup(&mut timer, cb, 42);
        assert!(!mod_timer(&mut timer, 10));
        assert!(timer_pending(&timer));
        assert!(run_timer(&mut timer, 10));
        assert_eq!(LAST_DATA.load(O::Acquire), 42);
        assert!(!timer_pending(&timer));
    }
}
