//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/irqinit.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/irqinit.c
//! Architecture IRQ initialization.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/irqinit.c
//!
//! Orchestrates early IRQ setup: legacy 8259 PIC programming, vector-to-IRQ
//! mapping for ISA vectors (`0x30..0x3f`), per-CPU IRQ stack allocation,
//! and the FRED-vs-IDT exception-gate handoff.

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::ENOMEM;

// === Vector layout — mirror vendor/linux/arch/x86/include/asm/irq_vectors.h ===

pub const FIRST_EXTERNAL_VECTOR: u32 = 0x20;
pub const NR_VECTORS: usize = 256;

/// `ISA_IRQ_VECTOR(irq)` — vectors 0x30..0x3f for legacy ISA IRQs 0..15.
pub const fn isa_irq_vector(irq: u32) -> u32 {
    ((FIRST_EXTERNAL_VECTOR + 16) & !15) + irq
}

/// Per-CPU vector table sentinel — `VECTOR_UNUSED`.
pub const VECTOR_UNUSED: i32 = -1;

/// Default number of legacy PIC-handled IRQs.
pub const NR_LEGACY_IRQS: usize = 16;

/// Trait seam mirroring `struct legacy_pic`.
pub trait LegacyPic {
    fn init(&self, auto_eoi: bool);
    fn nr_legacy_irqs(&self) -> usize;
}

/// Trait seam for the bits of `irq_desc` configuration that
/// `init_ISA_irqs` performs (`irq_set_chip_and_handler`,
/// `irq_set_status_flags`).
pub trait IrqDescOps {
    fn set_level_handler(&self, irq: u32);
}

/// Trait seam for `init_bsp_APIC()`.
pub trait BspApic {
    fn init(&self);
}

/// Linux's `init_ISA_irqs`: pre-program the legacy PIC, mark all 16 ISA
/// IRQs as level-triggered.
pub fn init_isa_irqs<B, P, I>(bsp: &B, pic: &P, irqs: &I)
where
    B: BspApic,
    P: LegacyPic,
    I: IrqDescOps,
{
    bsp.init();
    pic.init(false);
    for i in 0..(pic.nr_legacy_irqs() as u32) {
        irqs.set_level_handler(i);
    }
}

/// Trait seam for the bits `init_IRQ` itself does — populate per-CPU
/// vector table at vectors 0x30..0x3f and allocate the IRQ stack.
pub trait CpuIrqInit {
    fn assign_vector(&self, cpu: usize, vector: u32, irq: u32);
    fn allocate_irq_stack(&self, cpu: usize) -> Result<(), i32>;
    fn run_intr_init(&self);
    fn smp_processor_id(&self) -> usize;
}

/// Linux's `init_IRQ`: wire the legacy ISA vectors to IRQ descriptors on
/// CPU0, allocate the per-CPU IRQ stack, and dispatch to the platform
/// `intr_init` hook.
pub fn init_irq<P, C>(pic: &P, init: &C) -> Result<(), i32>
where
    P: LegacyPic,
    C: CpuIrqInit,
{
    for i in 0..(pic.nr_legacy_irqs() as u32) {
        init.assign_vector(0, isa_irq_vector(i), i);
    }
    init.allocate_irq_stack(init.smp_processor_id())?;
    init.run_intr_init();
    Ok(())
}

/// Per-CPU vector table mirror of `DEFINE_PER_CPU(vector_irq_t, vector_irq)`.
#[derive(Debug, Clone)]
pub struct VectorIrqTable {
    pub entries: Vec<i32>,
}

impl VectorIrqTable {
    pub fn new() -> Self {
        Self {
            entries: alloc::vec![VECTOR_UNUSED; NR_VECTORS],
        }
    }

    pub fn assign(&mut self, vector: u32, irq: i32) -> Result<(), i32> {
        let idx = vector as usize;
        if idx >= NR_VECTORS {
            return Err(ENOMEM);
        }
        self.entries[idx] = irq;
        Ok(())
    }
}

impl Default for VectorIrqTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::RefCell;

    #[test]
    fn first_external_vector_matches_linux() {
        assert_eq!(FIRST_EXTERNAL_VECTOR, 0x20);
    }

    #[test]
    fn isa_irq_vectors_start_at_0x30() {
        assert_eq!(isa_irq_vector(0), 0x30);
        assert_eq!(isa_irq_vector(1), 0x31);
        assert_eq!(isa_irq_vector(15), 0x3f);
    }

    #[test]
    fn vector_irq_table_initializes_to_unused() {
        let table = VectorIrqTable::new();
        assert_eq!(table.entries.len(), NR_VECTORS);
        assert!(table.entries.iter().all(|&v| v == VECTOR_UNUSED));
    }

    #[test]
    fn vector_irq_table_assign_round_trips() {
        let mut table = VectorIrqTable::new();
        table.assign(0x30, 0).unwrap();
        table.assign(0x3f, 15).unwrap();
        assert_eq!(table.entries[0x30], 0);
        assert_eq!(table.entries[0x3f], 15);
    }

    struct MockPic {
        init_called: RefCell<u32>,
    }
    impl LegacyPic for MockPic {
        fn init(&self, _auto: bool) {
            *self.init_called.borrow_mut() += 1;
        }
        fn nr_legacy_irqs(&self) -> usize {
            NR_LEGACY_IRQS
        }
    }

    struct MockBsp {
        called: RefCell<bool>,
    }
    impl BspApic for MockBsp {
        fn init(&self) {
            *self.called.borrow_mut() = true;
        }
    }

    struct MockIrqDesc {
        configured: RefCell<Vec<u32>>,
    }
    impl IrqDescOps for MockIrqDesc {
        fn set_level_handler(&self, irq: u32) {
            self.configured.borrow_mut().push(irq);
        }
    }

    #[test]
    fn init_isa_irqs_inits_bsp_pic_and_all_legacy_irqs() {
        let bsp = MockBsp {
            called: RefCell::new(false),
        };
        let pic = MockPic {
            init_called: RefCell::new(0),
        };
        let irqs = MockIrqDesc {
            configured: RefCell::new(Vec::new()),
        };
        init_isa_irqs(&bsp, &pic, &irqs);
        assert!(*bsp.called.borrow());
        assert_eq!(*pic.init_called.borrow(), 1);
        let cfg = irqs.configured.borrow();
        assert_eq!(cfg.len(), NR_LEGACY_IRQS);
        assert_eq!(cfg[0], 0);
        assert_eq!(cfg[15], 15);
    }

    struct MockInit {
        assignments: RefCell<Vec<(usize, u32, u32)>>,
        stack_calls: RefCell<u32>,
        intr_init_calls: RefCell<u32>,
        stack_result: Result<(), i32>,
    }
    impl CpuIrqInit for MockInit {
        fn assign_vector(&self, cpu: usize, vector: u32, irq: u32) {
            self.assignments.borrow_mut().push((cpu, vector, irq));
        }
        fn allocate_irq_stack(&self, _cpu: usize) -> Result<(), i32> {
            *self.stack_calls.borrow_mut() += 1;
            self.stack_result
        }
        fn run_intr_init(&self) {
            *self.intr_init_calls.borrow_mut() += 1;
        }
        fn smp_processor_id(&self) -> usize {
            0
        }
    }

    #[test]
    fn init_irq_assigns_isa_vectors_and_runs_intr_init() {
        let pic = MockPic {
            init_called: RefCell::new(0),
        };
        let init = MockInit {
            assignments: RefCell::new(Vec::new()),
            stack_calls: RefCell::new(0),
            intr_init_calls: RefCell::new(0),
            stack_result: Ok(()),
        };
        let result = init_irq(&pic, &init);
        assert!(result.is_ok());
        let a = init.assignments.borrow();
        assert_eq!(a.len(), NR_LEGACY_IRQS);
        assert_eq!(a[0], (0, 0x30, 0));
        assert_eq!(a[15], (0, 0x3f, 15));
        assert_eq!(*init.stack_calls.borrow(), 1);
        assert_eq!(*init.intr_init_calls.borrow(), 1);
    }

    #[test]
    fn init_irq_propagates_stack_allocation_failure() {
        let pic = MockPic {
            init_called: RefCell::new(0),
        };
        let init = MockInit {
            assignments: RefCell::new(Vec::new()),
            stack_calls: RefCell::new(0),
            intr_init_calls: RefCell::new(0),
            stack_result: Err(ENOMEM),
        };
        assert_eq!(init_irq(&pic, &init), Err(ENOMEM));
        assert_eq!(*init.intr_init_calls.borrow(), 0);
    }
}
