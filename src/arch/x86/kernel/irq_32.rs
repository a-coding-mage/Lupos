//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/irq_32.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/irq_32.c
//! 32-bit IRQ stack switching and stack-overflow detection.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/irq_32.c
//!
//! 32-bit kernels switch to a dedicated per-CPU IRQ stack when entering a
//! hardirq from the task stack, both to reduce footprint on the task
//! stack and to enable softirq-on-own-stack. Lupos targets x86_64, so the
//! actual asm `xchgl` swap is not used in production — but the algorithmic
//! decisions (when to switch, how to detect overflow) are kernel-ABI
//! relevant and faithfully ported here.

#![allow(dead_code)]

use crate::include::uapi::errno::ENOMEM;

/// Default kernel thread-stack size on 32-bit x86 (8 KiB).
pub const THREAD_SIZE: usize = 8 * 1024;

/// Warn threshold below which the IRQ entry treats the task stack as
/// "low" — Linux mirrors `sizeof(struct thread_info) + STACK_WARN`.
pub const THREAD_INFO_SIZE: usize = 64;
pub const STACK_WARN: usize = 1024;

/// Bytes free below which `check_stack_overflow` reports the stack low.
pub const STACK_LOW_THRESHOLD: usize = THREAD_INFO_SIZE + STACK_WARN;

/// IRQ-stack header layout. Each per-CPU IRQ stack stores the saved
/// previous `esp` at its base so the return-from-irq epilogue can switch
/// back to the task stack.
#[derive(Debug, Clone, Copy)]
pub struct IrqStack {
    pub base: u64,
    pub top: u64,
}

impl IrqStack {
    pub const fn new(base: u64) -> Self {
        Self {
            base,
            top: base + THREAD_SIZE as u64,
        }
    }

    /// `*prev_esp` slot — the bottom-of-IRQ-stack save location.
    pub fn prev_esp_slot(&self) -> u64 {
        self.base
    }
}

/// Linux's `check_stack_overflow`: is the current `esp` within
/// `STACK_LOW_THRESHOLD` of the bottom of the task stack?
///
/// `sp_in_page` is the value of `current_stack_pointer & (THREAD_SIZE-1)`.
pub fn check_stack_overflow(sp_in_page: usize) -> bool {
    sp_in_page < STACK_LOW_THRESHOLD
}

/// Linux's `execute_on_irq_stack`: returns true when the IRQ should be
/// dispatched on the dedicated IRQ stack. Returns false when the current
/// stack already *is* the IRQ stack (nested hardirq).
///
/// `current_stack_base` is the task stack base (page-aligned); `irq_stack`
/// is the per-CPU IRQ stack record.
pub fn execute_on_irq_stack(current_stack_base: u64, irq_stack: &IrqStack) -> bool {
    current_stack_base != irq_stack.base
}

/// Linux's `irq_init_percpu_irqstack`: allocate a hardirq and softirq
/// stack page for `cpu`. Trait seam to a real page allocator; production
/// wires this to the buddy allocator.
pub trait PageAllocator {
    fn alloc_thread_stack(&self) -> Option<u64>;
    fn free_thread_stack(&self, base: u64);
}

/// Per-CPU storage for hardirq / softirq stacks.
#[derive(Debug, Default, Clone, Copy)]
pub struct PerCpuIrqStacks {
    pub hardirq: Option<IrqStack>,
    pub softirq: Option<IrqStack>,
}

impl PerCpuIrqStacks {
    pub const fn new() -> Self {
        Self {
            hardirq: None,
            softirq: None,
        }
    }
}

/// Allocate both hardirq and softirq stacks for the given CPU's slot.
/// Mirrors `irq_init_percpu_irqstack`. Idempotent — returns Ok(()) without
/// reallocation if the hardirq stack is already populated.
pub fn irq_init_percpu_irqstack<A: PageAllocator>(
    alloc: &A,
    slot: &mut PerCpuIrqStacks,
) -> Result<(), i32> {
    if slot.hardirq.is_some() {
        return Ok(());
    }
    let hardirq_base = alloc.alloc_thread_stack().ok_or(ENOMEM)?;
    let softirq_base = match alloc.alloc_thread_stack() {
        Some(b) => b,
        None => {
            alloc.free_thread_stack(hardirq_base);
            return Err(ENOMEM);
        }
    };
    slot.hardirq = Some(IrqStack::new(hardirq_base));
    slot.softirq = Some(IrqStack::new(softirq_base));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::Cell;

    struct CountingAlloc {
        next: Cell<u64>,
        budget: Cell<u32>,
        freed: Cell<u32>,
    }

    impl PageAllocator for CountingAlloc {
        fn alloc_thread_stack(&self) -> Option<u64> {
            if self.budget.get() == 0 {
                return None;
            }
            self.budget.set(self.budget.get() - 1);
            let base = self.next.get();
            self.next.set(base + THREAD_SIZE as u64);
            Some(base)
        }
        fn free_thread_stack(&self, _base: u64) {
            self.freed.set(self.freed.get() + 1);
        }
    }

    #[test]
    fn stack_overflow_threshold_matches_linux() {
        // sizeof(struct thread_info) + STACK_WARN
        assert_eq!(STACK_LOW_THRESHOLD, 64 + 1024);
    }

    #[test]
    fn overflow_predicate_fires_below_threshold() {
        assert!(check_stack_overflow(500));
        assert!(!check_stack_overflow(THREAD_SIZE - 1));
    }

    #[test]
    fn execute_on_irq_stack_returns_false_when_already_on_irq_stack() {
        let irq = IrqStack::new(0x1000);
        assert!(!execute_on_irq_stack(0x1000, &irq));
    }

    #[test]
    fn execute_on_irq_stack_returns_true_when_on_task_stack() {
        let irq = IrqStack::new(0x1000);
        assert!(execute_on_irq_stack(0x9000, &irq));
    }

    #[test]
    fn irq_init_allocates_both_stacks() {
        let alloc = CountingAlloc {
            next: Cell::new(0x10000),
            budget: Cell::new(8),
            freed: Cell::new(0),
        };
        let mut slot = PerCpuIrqStacks::new();
        let r = irq_init_percpu_irqstack(&alloc, &mut slot);
        assert!(r.is_ok());
        assert!(slot.hardirq.is_some());
        assert!(slot.softirq.is_some());
        assert_eq!(alloc.freed.get(), 0);
    }

    #[test]
    fn irq_init_frees_hardirq_when_softirq_alloc_fails() {
        let alloc = CountingAlloc {
            next: Cell::new(0x10000),
            budget: Cell::new(1),
            freed: Cell::new(0),
        };
        let mut slot = PerCpuIrqStacks::new();
        assert_eq!(irq_init_percpu_irqstack(&alloc, &mut slot), Err(ENOMEM));
        assert_eq!(alloc.freed.get(), 1);
    }

    #[test]
    fn irq_init_is_idempotent() {
        let alloc = CountingAlloc {
            next: Cell::new(0x10000),
            budget: Cell::new(8),
            freed: Cell::new(0),
        };
        let mut slot = PerCpuIrqStacks::new();
        irq_init_percpu_irqstack(&alloc, &mut slot).unwrap();
        let first = slot.hardirq.unwrap().base;
        irq_init_percpu_irqstack(&alloc, &mut slot).unwrap();
        assert_eq!(slot.hardirq.unwrap().base, first);
    }
}
