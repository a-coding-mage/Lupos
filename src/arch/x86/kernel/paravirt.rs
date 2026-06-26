//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/kernel/paravirt.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/paravirt.c
//! Native x86 paravirt operation table.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/paravirt.c

#![allow(dead_code)]

use spin::Mutex;

pub const NATIVE_BANNER: &str = "Booting paravirtualized kernel on bare hardware";
pub const X86_EFLAGS_IF: u64 = 1 << 9;

pub type NotifyPageEncStatusChanged = fn(pfn: u64, npages: usize, enc: bool);

/// MMU paravirt ops subset used by x86 encryption transitions.
///
/// Source:
/// - `vendor/linux/arch/x86/include/asm/paravirt_types.h`
/// - `vendor/linux/arch/x86/kernel/paravirt.c`
#[derive(Clone, Copy)]
pub struct PvMmuOps {
    pub notify_page_enc_status_changed: NotifyPageEncStatusChanged,
}

fn notify_page_enc_status_changed_noop(_pfn: u64, _npages: usize, _enc: bool) {}

impl PvMmuOps {
    pub const fn linux_noop() -> Self {
        Self {
            notify_page_enc_status_changed: notify_page_enc_status_changed_noop,
        }
    }
}

static PV_MMU_OPS: Mutex<PvMmuOps> = Mutex::new(PvMmuOps::linux_noop());

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PvInfo {
    pub name: &'static str,
    pub paravirt_enabled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeCpuState {
    pub rflags: u64,
    pub cr2: u64,
    pub cr3: u64,
    pub halted: bool,
}

impl Default for NativeCpuState {
    fn default() -> Self {
        Self {
            rflags: X86_EFLAGS_IF,
            cr2: 0,
            cr3: 0,
            halted: false,
        }
    }
}

pub const PV_INFO: PvInfo = PvInfo {
    name: "bare hardware",
    paravirt_enabled: false,
};

pub const fn default_banner() -> &'static str {
    NATIVE_BANNER
}

pub const fn native_save_fl(state: &NativeCpuState) -> u64 {
    state.rflags
}

pub fn native_irq_disable(state: &mut NativeCpuState) {
    state.rflags &= !X86_EFLAGS_IF;
}

pub fn native_irq_enable(state: &mut NativeCpuState) {
    state.rflags |= X86_EFLAGS_IF;
}

pub fn native_safe_halt(state: &mut NativeCpuState) {
    native_irq_enable(state);
    state.halted = true;
}

pub fn native_halt(state: &mut NativeCpuState) {
    state.halted = true;
}

pub const fn native_read_cr2(state: &NativeCpuState) -> u64 {
    state.cr2
}

pub fn native_write_cr3(state: &mut NativeCpuState, value: u64) {
    state.cr3 = value;
}

pub const fn native_read_cr3(state: &NativeCpuState) -> u64 {
    state.cr3
}

pub fn set_mmu_ops(ops: PvMmuOps) {
    *PV_MMU_OPS.lock() = ops;
}

pub fn reset_mmu_ops() {
    set_mmu_ops(PvMmuOps::linux_noop());
}

pub fn notify_page_enc_status_changed(pfn: u64, npages: usize, enc: bool) {
    let notify = PV_MMU_OPS.lock().notify_page_enc_status_changed;
    notify(pfn, npages, enc);
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use core::sync::atomic::{AtomicUsize, Ordering};

    const NOTIFY_LOG_CAP: usize = 8;
    static NOTIFY_LOG: Mutex<[(u64, usize, bool); NOTIFY_LOG_CAP]> =
        Mutex::new([(0, 0, false); NOTIFY_LOG_CAP]);
    static NOTIFY_LOG_LEN: AtomicUsize = AtomicUsize::new(0);

    fn record_notify_page_enc_status_changed(pfn: u64, npages: usize, enc: bool) {
        let idx = NOTIFY_LOG_LEN.fetch_add(1, Ordering::AcqRel);
        if idx < NOTIFY_LOG_CAP {
            NOTIFY_LOG.lock()[idx] = (pfn, npages, enc);
        }
    }

    pub fn reset_notify_page_enc_log() {
        NOTIFY_LOG_LEN.store(0, Ordering::Release);
        *NOTIFY_LOG.lock() = [(0, 0, false); NOTIFY_LOG_CAP];
    }

    pub fn notify_page_enc_log() -> [(u64, usize, bool); 1] {
        assert_eq!(
            NOTIFY_LOG_LEN.load(Ordering::Acquire),
            1,
            "test expected one paravirt page-encryption notification"
        );
        [NOTIFY_LOG.lock()[0]]
    }

    pub fn recording_mmu_ops() -> PvMmuOps {
        PvMmuOps {
            notify_page_enc_status_changed: record_notify_page_enc_status_changed,
        }
    }

    #[test]
    fn native_irq_ops_toggle_if_bit() {
        let mut s = NativeCpuState::default();
        assert_ne!(native_save_fl(&s) & X86_EFLAGS_IF, 0);
        native_irq_disable(&mut s);
        assert_eq!(s.rflags & X86_EFLAGS_IF, 0);
        native_irq_enable(&mut s);
        assert_ne!(s.rflags & X86_EFLAGS_IF, 0);
    }

    #[test]
    fn native_cr3_round_trip_and_halt() {
        let mut s = NativeCpuState::default();
        native_write_cr3(&mut s, 0x1234);
        assert_eq!(native_read_cr3(&s), 0x1234);
        native_safe_halt(&mut s);
        assert!(s.halted);
        assert_ne!(s.rflags & X86_EFLAGS_IF, 0);
    }

    #[test]
    fn mmu_notify_page_enc_status_changed_defaults_noop_and_can_record() {
        reset_mmu_ops();
        reset_notify_page_enc_log();
        notify_page_enc_status_changed(0x123, 2, true);
        assert_eq!(NOTIFY_LOG_LEN.load(Ordering::Acquire), 0);

        set_mmu_ops(recording_mmu_ops());
        notify_page_enc_status_changed(0x456, 3, false);
        assert_eq!(notify_page_enc_log(), [(0x456, 3, false)]);
        reset_mmu_ops();
    }
}
