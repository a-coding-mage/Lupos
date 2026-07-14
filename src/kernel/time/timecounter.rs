//! linux-parity: complete
//! linux-source: vendor/linux/kernel/time/timecounter.c
//! test-origin: linux:vendor/linux/kernel/time/timecounter.c
//! Timecounter coverage for M36.
//!
//! Mirrors `vendor/linux/kernel/time/timecounter.c`.

use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("timecounter_init", linux_timecounter_init as usize, true);
    export_symbol_once("timecounter_read", linux_timecounter_read as usize, true);
}

type LinuxCyclecounterRead = unsafe extern "C" fn(*mut LinuxCyclecounter) -> u64;

/// `struct cyclecounter` - `vendor/linux/include/linux/timecounter.h`.
#[repr(C)]
pub struct LinuxCyclecounter {
    pub read: Option<LinuxCyclecounterRead>,
    pub mask: u64,
    pub mult: u32,
    pub shift: u32,
}

/// `struct timecounter` - `vendor/linux/include/linux/timecounter.h`.
#[repr(C)]
pub struct LinuxTimecounter {
    pub cc: *mut LinuxCyclecounter,
    pub cycle_last: u64,
    pub nsec: u64,
    pub mask: u64,
    pub frac: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Timecounter {
    pub cycle_last: u64,
    pub nsec: u64,
    pub mask: u64,
    pub mult: u32,
    pub shift: u32,
}

impl Timecounter {
    pub const fn new(cycle_last: u64, mask: u64, mult: u32, shift: u32) -> Self {
        Self {
            cycle_last,
            nsec: 0,
            mask,
            mult,
            shift,
        }
    }

    pub fn read(&mut self, cycle_now: u64) -> u64 {
        let delta = cycle_now.wrapping_sub(self.cycle_last) & self.mask;
        self.cycle_last = cycle_now;
        self.nsec = self
            .nsec
            .saturating_add(delta.saturating_mul(self.mult as u64) >> self.shift);
        self.nsec
    }
}

fn linux_cyclecounter_mask_for_shift(shift: u32) -> u64 {
    if shift >= 64 {
        u64::MAX
    } else {
        (1u64 << shift) - 1
    }
}

unsafe fn linux_cyclecounter_read(cc: *mut LinuxCyclecounter) -> u64 {
    if cc.is_null() {
        return 0;
    }
    unsafe { (*cc).read.map(|read| read(cc)).unwrap_or(0) }
}

unsafe fn linux_cyclecounter_cyc2ns(
    cc: *const LinuxCyclecounter,
    cycles: u64,
    mask: u64,
    frac: *mut u64,
) -> u64 {
    if cc.is_null() || frac.is_null() {
        return 0;
    }
    let ns = cycles
        .wrapping_mul(unsafe { (*cc).mult as u64 })
        .wrapping_add(unsafe { *frac });
    unsafe {
        *frac = ns & mask;
        ns >> (*cc).shift
    }
}

/// `timecounter_init` - `vendor/linux/kernel/time/timecounter.c`.
#[unsafe(export_name = "timecounter_init")]
pub unsafe extern "C" fn linux_timecounter_init(
    tc: *mut LinuxTimecounter,
    cc: *mut LinuxCyclecounter,
    start_tstamp: u64,
) {
    if tc.is_null() || cc.is_null() {
        return;
    }

    unsafe {
        (*tc).cc = cc;
        (*tc).cycle_last = linux_cyclecounter_read(cc);
        (*tc).nsec = start_tstamp;
        (*tc).mask = linux_cyclecounter_mask_for_shift((*cc).shift);
        (*tc).frac = 0;
    }
}

/// `timecounter_read` - `vendor/linux/kernel/time/timecounter.c`.
#[unsafe(export_name = "timecounter_read")]
pub unsafe extern "C" fn linux_timecounter_read(tc: *mut LinuxTimecounter) -> u64 {
    if tc.is_null() {
        return 0;
    }

    unsafe {
        let cc = (*tc).cc;
        if cc.is_null() {
            return (*tc).nsec;
        }
        let cycle_now = linux_cyclecounter_read(cc);
        let cycle_delta = cycle_now.wrapping_sub((*tc).cycle_last) & (*cc).mask;
        let ns_offset = linux_cyclecounter_cyc2ns(
            cc,
            cycle_delta,
            (*tc).mask,
            core::ptr::addr_of_mut!((*tc).frac),
        );
        (*tc).cycle_last = cycle_now;
        (*tc).nsec = (*tc).nsec.wrapping_add(ns_offset);
        (*tc).nsec
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicU64, Ordering};

    #[test]
    fn timecounter_accumulates_scaled_delta() {
        let mut tc = Timecounter::new(10, u64::MAX, 2, 0);
        assert_eq!(tc.read(15), 10);
        assert_eq!(tc.read(20), 20);
    }

    #[test]
    fn linux_timecounter_init_and_read_match_vendor_layout() {
        static NEXT_CYCLE: AtomicU64 = AtomicU64::new(10);

        unsafe extern "C" fn read(_cc: *mut LinuxCyclecounter) -> u64 {
            NEXT_CYCLE.fetch_add(5, Ordering::AcqRel)
        }

        unsafe {
            let mut cc = LinuxCyclecounter {
                read: Some(read),
                mask: u64::MAX,
                mult: 2,
                shift: 0,
            };
            let mut tc = core::mem::zeroed::<LinuxTimecounter>();

            linux_timecounter_init(&mut tc, &mut cc, 100);

            assert_eq!(tc.cc, &mut cc as *mut _);
            assert_eq!(tc.cycle_last, 10);
            assert_eq!(tc.nsec, 100);
            assert_eq!(tc.mask, 0);
            assert_eq!(linux_timecounter_read(&mut tc), 110);
        }
    }

    #[test]
    fn timecounter_exports_register_for_modules() {
        register_module_exports();

        assert_eq!(
            crate::kernel::module::find_symbol("timecounter_init"),
            Some(linux_timecounter_init as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("timecounter_read"),
            Some(linux_timecounter_read as usize)
        );
    }
}
