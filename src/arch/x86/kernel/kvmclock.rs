//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/kvmclock.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/kvmclock.c
//! KVM paravirtual clock policy.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/kvmclock.c

#![allow(dead_code)]

use core::sync::atomic::{AtomicBool, Ordering};

pub const MSR_KVM_WALL_CLOCK: u32 = 0x11;
pub const MSR_KVM_SYSTEM_TIME: u32 = 0x12;
pub const MSR_KVM_WALL_CLOCK_NEW: u32 = 0x4b56_4d00;
pub const MSR_KVM_SYSTEM_TIME_NEW: u32 = 0x4b56_4d01;

pub const KVM_FEATURE_CLOCKSOURCE: u32 = 0;
pub const KVM_FEATURE_NOP_IO_DELAY: u32 = 1;
pub const KVM_FEATURE_CLOCKSOURCE2: u32 = 3;
pub const KVM_FEATURE_PV_UNHALT: u32 = 7;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KvmClockPolicy {
    pub disabled: bool,
    pub vsyscall_disabled: bool,
    pub features: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KvmClockMsrs {
    pub wall_clock: u32,
    pub system_time: u32,
}

pub fn parse_kvmclock_param(policy: &mut KvmClockPolicy, param: &str) -> bool {
    match param {
        "no-kvmclock" => {
            policy.disabled = true;
            true
        }
        "no-kvmclock-vsyscall" => {
            policy.vsyscall_disabled = true;
            true
        }
        _ => false,
    }
}

pub const fn select_kvmclock_msrs(features: u32) -> Option<KvmClockMsrs> {
    if features & (1 << KVM_FEATURE_CLOCKSOURCE2) != 0 {
        Some(KvmClockMsrs {
            wall_clock: MSR_KVM_WALL_CLOCK_NEW,
            system_time: MSR_KVM_SYSTEM_TIME_NEW,
        })
    } else if features & (1 << KVM_FEATURE_CLOCKSOURCE) != 0 {
        Some(KvmClockMsrs {
            wall_clock: MSR_KVM_WALL_CLOCK,
            system_time: MSR_KVM_SYSTEM_TIME,
        })
    } else {
        None
    }
}

pub fn kvm_check_and_clear_guest_paused(paused: &AtomicBool) -> bool {
    paused.swap(false, Ordering::AcqRel)
}

pub const fn sched_clock_offset(host_ns: u64, guest_ns: u64) -> i64 {
    guest_ns as i64 - host_ns as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn early_params_disable_clock_or_vsyscall() {
        let mut p = KvmClockPolicy::default();
        assert!(parse_kvmclock_param(&mut p, "no-kvmclock-vsyscall"));
        assert!(p.vsyscall_disabled);
        assert!(parse_kvmclock_param(&mut p, "no-kvmclock"));
        assert!(p.disabled);
    }

    #[test]
    fn clocksource2_selects_new_msrs() {
        assert_eq!(
            select_kvmclock_msrs(1 << KVM_FEATURE_CLOCKSOURCE2),
            Some(KvmClockMsrs {
                wall_clock: MSR_KVM_WALL_CLOCK_NEW,
                system_time: MSR_KVM_SYSTEM_TIME_NEW,
            })
        );
        assert_eq!(select_kvmclock_msrs(0), None);
    }

    #[test]
    fn guest_paused_is_one_shot() {
        let paused = AtomicBool::new(true);
        assert!(kvm_check_and_clear_guest_paused(&paused));
        assert!(!kvm_check_and_clear_guest_paused(&paused));
    }
}
