//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/irq_work.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/irq_work.c
//! x86 IRQ work APIC vector.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/irq_work.c
//!
//! Linux ref: Documentation/core-api/irq/irq.rst
//! Intel SDM Vol. 3 §10.6 — "Issuing Interprocessor Interrupts"

#![allow(dead_code)]

/// Vector for the `irq_work` self-IPI.
///
/// Mirrors `IRQ_WORK_VECTOR` in
/// `vendor/linux/arch/x86/include/asm/irq_vectors.h` (0xf6).
pub const IRQ_WORK_VECTOR: u8 = 0xf6;

/// Returns whether the local APIC is required to deliver irq_work IPIs.
///
/// Linux gates the entire `sysvec_irq_work` IDT entry on
/// `CONFIG_X86_LOCAL_APIC`; lupos always has APIC support compiled in but
/// we expose the predicate so callers can mirror the gate.
pub fn arch_irq_work_has_interrupt() -> bool {
    true
}

/// Trait seam for the APIC self-IPI used by `arch_irq_work_raise`.
///
/// Tests use a recording mock; production wires this to the real APIC
/// in `crate::arch::x86::kernel::apic`.
pub trait ApicSelfIpi {
    fn send_self(&self, vector: u8);
    fn wait_icr_idle(&self);
}

/// Linux's `arch_irq_work_raise`: send a self-IPI on `IRQ_WORK_VECTOR`
/// and wait for the ICR to become idle.
pub fn arch_irq_work_raise<A: ApicSelfIpi>(apic: &A) {
    if !arch_irq_work_has_interrupt() {
        return;
    }
    apic.send_self(IRQ_WORK_VECTOR);
    apic.wait_icr_idle();
}

/// Trait seam for `irq_work_run` (the generic kernel-side worker
/// dispatch). The arch IDT entry forwards to this.
pub trait IrqWorkRunner {
    fn run(&self);
}

/// Bookkeeping counter mirroring `apic_irq_work_irqs` in `irq_cpustat_t`.
#[derive(Default, Debug, Clone, Copy)]
pub struct IrqWorkStat {
    pub count: u64,
}

impl IrqWorkStat {
    pub fn inc(&mut self) {
        self.count = self.count.wrapping_add(1);
    }
}

/// Linux's `sysvec_irq_work` IDT entry: ack APIC, bump stat, run pending
/// work. Caller is responsible for the IDT plumbing; this function carries
/// the per-vector body.
pub fn sysvec_irq_work_body<A, R>(apic: &A, stat: &mut IrqWorkStat, runner: &R)
where
    A: ApicEoi,
    R: IrqWorkRunner,
{
    apic.eoi();
    stat.inc();
    runner.run();
}

/// Local APIC end-of-interrupt seam (`apic_eoi`).
pub trait ApicEoi {
    fn eoi(&self);
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::Cell;

    struct MockApic {
        sent: Cell<Option<u8>>,
        wait_calls: Cell<u32>,
        eoi_calls: Cell<u32>,
    }

    impl MockApic {
        fn new() -> Self {
            Self {
                sent: Cell::new(None),
                wait_calls: Cell::new(0),
                eoi_calls: Cell::new(0),
            }
        }
    }

    impl ApicSelfIpi for MockApic {
        fn send_self(&self, vector: u8) {
            self.sent.set(Some(vector));
        }
        fn wait_icr_idle(&self) {
            self.wait_calls.set(self.wait_calls.get() + 1);
        }
    }

    impl ApicEoi for MockApic {
        fn eoi(&self) {
            self.eoi_calls.set(self.eoi_calls.get() + 1);
        }
    }

    struct CountingRunner(Cell<u32>);
    impl IrqWorkRunner for CountingRunner {
        fn run(&self) {
            self.0.set(self.0.get() + 1);
        }
    }

    #[test]
    fn irq_work_vector_matches_linux() {
        assert_eq!(IRQ_WORK_VECTOR, 0xf6);
    }

    #[test]
    fn raise_sends_self_ipi_on_vector_and_waits() {
        let apic = MockApic::new();
        arch_irq_work_raise(&apic);
        assert_eq!(apic.sent.get(), Some(IRQ_WORK_VECTOR));
        assert_eq!(apic.wait_calls.get(), 1);
    }

    #[test]
    fn sysvec_body_eois_increments_stat_runs_work() {
        let apic = MockApic::new();
        let mut stat = IrqWorkStat::default();
        let runner = CountingRunner(Cell::new(0));
        sysvec_irq_work_body(&apic, &mut stat, &runner);
        assert_eq!(apic.eoi_calls.get(), 1);
        assert_eq!(stat.count, 1);
        assert_eq!(runner.0.get(), 1);
    }
}
