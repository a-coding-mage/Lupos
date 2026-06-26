//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/mce/threshold.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/mce/threshold.c
//! Corrected MCE threshold and CMCI storm tracking.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/cpu/mce/threshold.c

use ::core::sync::atomic::{AtomicU32, Ordering};

use super::amd;
use super::core::{
    MAX_NR_BANKS, MCI_STATUS_VAL, Mce, MceBankSet, MceVendorFlags, mce_is_correctable,
};
use crate::arch::x86::kernel::cpu::CpuVendor;
use crate::include::uapi::errno::EINVAL;

pub const NUM_HISTORY_BITS: u32 = 64;
pub const STORM_BEGIN_THRESHOLD: u32 = 5;
pub const STORM_END_POLL_THRESHOLD: u32 = 29;
pub const THRESHOLD_APIC_VECTOR: u8 = 0xf0;

static MCE_APEI_THR_LIMIT: AtomicU32 = AtomicU32::new(0);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StormBank {
    pub history: u64,
    pub timestamp: u64,
    pub in_storm_mode: bool,
    pub poll_only: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct McaStormDesc {
    pub banks: [StormBank; MAX_NR_BANKS],
    pub stormy_bank_count: u8,
    pub poll_mode: bool,
    pub poll_banks: MceBankSet,
}

impl Default for McaStormDesc {
    fn default() -> Self {
        Self {
            banks: [StormBank::default(); MAX_NR_BANKS],
            stormy_bank_count: 0,
            poll_mode: false,
            poll_banks: MceBankSet::empty(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StormAction {
    None,
    Begin {
        bank: usize,
        vendor_action: Option<amd::AmdStormAction>,
    },
    End {
        bank: usize,
        vendor_action: Option<amd::AmdStormAction>,
    },
}

pub fn mce_save_apei_thr_limit(thr_limit: u32) {
    MCE_APEI_THR_LIMIT.store(thr_limit, Ordering::Release);
}

pub fn mce_get_apei_thr_limit() -> u32 {
    MCE_APEI_THR_LIMIT.load(Ordering::Acquire)
}

pub fn threshold_limit_from_apei() -> u16 {
    let limit = mce_get_apei_thr_limit();
    if limit == 0 {
        amd::THRESHOLD_MAX
    } else {
        limit.min(amd::THRESHOLD_MAX as u32) as u16
    }
}

pub fn mce_inherit_storm(desc: &mut McaStormDesc, bank: usize, now: u64) -> Result<(), i32> {
    let Some(slot) = desc.banks.get_mut(bank) else {
        return Err(EINVAL);
    };
    slot.history = u64::MAX;
    slot.timestamp = now;
    Ok(())
}

pub const fn mce_get_storm_mode(desc: &McaStormDesc) -> bool {
    desc.poll_mode
}

pub fn mce_set_storm_mode(desc: &mut McaStormDesc, storm: bool) {
    desc.poll_mode = storm;
}

pub fn cmci_storm_begin(desc: &mut McaStormDesc, bank: usize) -> Result<(), i32> {
    let Some(slot) = desc.banks.get_mut(bank) else {
        return Err(EINVAL);
    };
    desc.poll_banks.set(bank)?;
    if !slot.in_storm_mode {
        slot.in_storm_mode = true;
        desc.stormy_bank_count = desc.stormy_bank_count.saturating_add(1);
    }
    desc.poll_mode = desc.stormy_bank_count != 0;
    Ok(())
}

pub fn cmci_storm_end(
    desc: &mut McaStormDesc,
    bank: usize,
    flags: MceVendorFlags,
) -> Result<(), i32> {
    let Some(slot) = desc.banks.get_mut(bank) else {
        return Err(EINVAL);
    };
    if !flags.amd_threshold {
        desc.poll_banks.clear(bank)?;
    }
    slot.history = 0;
    if slot.in_storm_mode {
        slot.in_storm_mode = false;
        desc.stormy_bank_count = desc.stormy_bank_count.saturating_sub(1);
    }
    desc.poll_mode = desc.stormy_bank_count != 0;
    Ok(())
}

fn vendor_action(vendor: CpuVendor, bank: usize, on: bool) -> Option<amd::AmdStormAction> {
    match vendor {
        CpuVendor::Amd => Some(amd::mce_amd_handle_storm(bank, on)),
        _ => None,
    }
}

pub fn mce_track_storm(
    desc: &mut McaStormDesc,
    mce: &Mce,
    now: u64,
    flags: MceVendorFlags,
) -> Result<StormAction, i32> {
    let bank = mce.bank as usize;
    let Some(slot) = desc.banks.get_mut(bank) else {
        return Err(EINVAL);
    };
    if slot.poll_only {
        return Ok(StormAction::None);
    }

    let mut shift = 1;
    if !slot.in_storm_mode {
        let delta = now.saturating_sub(slot.timestamp);
        shift = (delta + 1).max(1);
    }
    let mut history = if shift < NUM_HISTORY_BITS as u64 {
        slot.history << shift
    } else {
        0
    };
    slot.timestamp = now;
    if (mce.status & MCI_STATUS_VAL) != 0 && mce_is_correctable(mce) {
        history |= 1;
    }
    slot.history = history;

    if slot.in_storm_mode {
        if (history & ((1u64 << (STORM_END_POLL_THRESHOLD + 1)) - 1)) != 0 {
            return Ok(StormAction::None);
        }
        let action = vendor_action(mce.cpuvendor, bank, false);
        cmci_storm_end(desc, bank, flags)?;
        Ok(StormAction::End {
            bank,
            vendor_action: action,
        })
    } else if history.count_ones() >= STORM_BEGIN_THRESHOLD {
        let action = vendor_action(mce.cpuvendor, bank, true);
        cmci_storm_begin(desc, bank)?;
        Ok(StormAction::Begin {
            bank,
            vendor_action: action,
        })
    } else {
        Ok(StormAction::None)
    }
}

#[cfg(test)]
mod tests {
    use super::super::core::{MCI_STATUS_VAL, Mce};
    use super::*;

    #[test]
    fn apei_threshold_limit_is_saved_and_clamped() {
        mce_save_apei_thr_limit(42);
        assert_eq!(mce_get_apei_thr_limit(), 42);
        assert_eq!(threshold_limit_from_apei(), 42);
        mce_save_apei_thr_limit(0xffff);
        assert_eq!(threshold_limit_from_apei(), amd::THRESHOLD_MAX);
    }

    #[test]
    fn storm_begin_and_end_update_poll_state() {
        let mut desc = McaStormDesc::default();
        cmci_storm_begin(&mut desc, 2).unwrap();
        assert!(desc.banks[2].in_storm_mode);
        assert!(desc.poll_banks.contains(2));
        assert!(mce_get_storm_mode(&desc));

        cmci_storm_end(&mut desc, 2, MceVendorFlags::default()).unwrap();
        assert!(!desc.banks[2].in_storm_mode);
        assert!(!desc.poll_banks.contains(2));
        assert!(!mce_get_storm_mode(&desc));
    }

    #[test]
    fn storm_tracking_detects_corrected_error_burst() {
        let mut desc = McaStormDesc::default();
        let m = Mce {
            cpuvendor: CpuVendor::Intel,
            bank: 1,
            status: MCI_STATUS_VAL,
            ..Mce::default()
        };
        let mut action = StormAction::None;
        for now in 0..5 {
            action = mce_track_storm(&mut desc, &m, now, MceVendorFlags::default()).unwrap();
        }
        assert_eq!(
            action,
            StormAction::Begin {
                bank: 1,
                vendor_action: None
            }
        );
    }

    #[test]
    fn storm_tracking_ends_after_empty_history_window() {
        let mut desc = McaStormDesc::default();
        cmci_storm_begin(&mut desc, 1).unwrap();
        desc.banks[1].history = 0;
        let m = Mce {
            cpuvendor: CpuVendor::Amd,
            bank: 1,
            status: 0,
            ..Mce::default()
        };
        assert_eq!(
            mce_track_storm(&mut desc, &m, 1, MceVendorFlags::default()).unwrap(),
            StormAction::End {
                bank: 1,
                vendor_action: Some(amd::AmdStormAction::DisableThresholdInterrupt { bank: 1 })
            }
        );
    }
}
