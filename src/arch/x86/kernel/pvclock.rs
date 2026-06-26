//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/pvclock.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/pvclock.c
//! KVM/Xen paravirtual clock helpers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/pvclock.c

#![allow(dead_code)]

use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

pub const PVCLOCK_TSC_STABLE_BIT: u8 = 1 << 0;
pub const NSEC_PER_SEC: u64 = 1_000_000_000;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PvclockVcpuTimeInfo {
    pub version: u32,
    pub pad0: u32,
    pub tsc_timestamp: u64,
    pub system_time: u64,
    pub tsc_to_system_mul: u32,
    pub tsc_shift: i8,
    pub flags: u8,
    pub pad: [u8; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PvclockWallClock {
    pub version: u32,
    pub sec: u32,
    pub nsec: u32,
}

static PVTI_CPU0_VA: AtomicUsize = AtomicUsize::new(0);

pub fn pvclock_tsc_khz(tsc_to_system_mul: u32, tsc_shift: i8) -> u64 {
    if tsc_to_system_mul == 0 {
        return 0;
    }
    let mut khz = ((1_000_000u128 << 32) / tsc_to_system_mul as u128) as u64;
    if tsc_shift < 0 {
        khz <<= (-tsc_shift) as u32;
    } else {
        khz >>= tsc_shift as u32;
    }
    khz
}

pub fn pvclock_scale_delta(delta: u64, mul: u32, shift: i8) -> u64 {
    let shifted = if shift < 0 {
        delta >> (-shift) as u32
    } else {
        delta << shift as u32
    };
    ((shifted as u128 * mul as u128) >> 32) as u64
}

pub fn pvclock_clocksource_read(info: &PvclockVcpuTimeInfo, tsc: u64, last: &AtomicU64) -> u64 {
    let delta = tsc.wrapping_sub(info.tsc_timestamp);
    let value = info.system_time.wrapping_add(pvclock_scale_delta(
        delta,
        info.tsc_to_system_mul,
        info.tsc_shift,
    ));
    loop {
        let old = last.load(Ordering::Acquire);
        if value <= old {
            return old;
        }
        if last
            .compare_exchange(old, value, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            return value;
        }
    }
}

pub const fn pvclock_read_flags(info: &PvclockVcpuTimeInfo) -> u8 {
    info.flags
}

pub fn pvclock_read_wallclock(wall: &PvclockWallClock, system_time_ns: u64) -> (u64, u32) {
    let mut sec = wall.sec as u64 + system_time_ns / NSEC_PER_SEC;
    let mut nsec = wall.nsec as u64 + system_time_ns % NSEC_PER_SEC;
    if nsec >= NSEC_PER_SEC {
        sec += 1;
        nsec -= NSEC_PER_SEC;
    }
    (sec, nsec as u32)
}

pub fn pvclock_set_pvti_cpu0_va(addr: usize) {
    PVTI_CPU0_VA.store(addr, Ordering::Release);
}

pub fn pvclock_get_pvti_cpu0_va() -> usize {
    PVTI_CPU0_VA.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tsc_khz_tracks_shift_direction() {
        let base = pvclock_tsc_khz(1 << 20, 0);
        assert_eq!(pvclock_tsc_khz(1 << 20, -1), base << 1);
        assert_eq!(pvclock_tsc_khz(1 << 20, 1), base >> 1);
    }

    #[test]
    fn clocksource_read_is_monotonic() {
        let info = PvclockVcpuTimeInfo {
            tsc_timestamp: 100,
            system_time: 1_000,
            tsc_to_system_mul: 1 << 20,
            ..Default::default()
        };
        let last = AtomicU64::new(2_000);
        assert_eq!(pvclock_clocksource_read(&info, 101, &last), 2_000);
        assert!(pvclock_clocksource_read(&info, 10_000_000, &last) >= 2_000);
    }

    #[test]
    fn wallclock_normalizes_nsec() {
        let wall = PvclockWallClock {
            sec: 10,
            nsec: 900_000_000,
            version: 0,
        };
        assert_eq!(
            pvclock_read_wallclock(&wall, 200_000_000),
            (11, 100_000_000)
        );
    }
}
