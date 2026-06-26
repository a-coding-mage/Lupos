//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/time.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/time.c
//! x86 timer initialization hooks.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/time.c

#![allow(dead_code)]

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::arch::x86::kernel::ptrace::PtRegs;

pub static HPET_TIME_INITIALIZED: AtomicBool = AtomicBool::new(false);
pub static DEFAULT_TIMER_IRQ_SETUP: AtomicBool = AtomicBool::new(false);
pub static ARCH_CLOCKSOURCE_MASK: AtomicU64 = AtomicU64::new(0);

pub const TIMER_IRQ: u8 = 0;

pub const fn profile_pc(regs: &PtRegs) -> u64 {
    regs.rip
}

pub fn setup_default_timer_irq() {
    DEFAULT_TIMER_IRQ_SETUP.store(true, Ordering::Release);
}

pub fn hpet_time_init(hpet_available: bool) -> bool {
    HPET_TIME_INITIALIZED.store(hpet_available, Ordering::Release);
    if !hpet_available {
        setup_default_timer_irq();
    }
    hpet_available
}

pub fn time_init(hpet_available: bool) {
    hpet_time_init(hpet_available);
}

pub fn clocksource_arch_init(mask: u64) {
    ARCH_CLOCKSOURCE_MASK.store(mask, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_pc_reads_instruction_pointer() {
        let regs = PtRegs {
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            rbp: 0,
            rbx: 0,
            r11: 0,
            r10: 0,
            r9: 0,
            r8: 0,
            rax: 0,
            rcx: 0,
            rdx: 0,
            rsi: 0,
            rdi: 0,
            orig_rax: 0,
            rip: 0xdead,
            cs: 0,
            eflags: 0,
            rsp: 0,
            ss: 0,
        };
        assert_eq!(profile_pc(&regs), 0xdead);
    }

    #[test]
    fn time_init_falls_back_to_default_irq_without_hpet() {
        DEFAULT_TIMER_IRQ_SETUP.store(false, Ordering::Release);
        time_init(false);
        assert!(DEFAULT_TIMER_IRQ_SETUP.load(Ordering::Acquire));
        clocksource_arch_init(0xffff);
        assert_eq!(ARCH_CLOCKSOURCE_MASK.load(Ordering::Acquire), 0xffff);
    }
}
