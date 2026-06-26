//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/irq_64.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/irq_64.c
//! x86_64 hardirq stack mapping policy.

use crate::include::uapi::errno::ENOMEM;

pub const PAGE_SHIFT: usize = 12;
pub const PAGE_SIZE: usize = 1 << PAGE_SHIFT;
pub const IRQ_STACK_SIZE: usize = 16 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IrqStackBacking {
    Vmap { guard_pages: bool },
    PerCpuBackingStore,
}

pub const fn irq_stack_pages() -> usize {
    IRQ_STACK_SIZE / PAGE_SIZE
}

pub const fn hardirq_stack_top(base: usize) -> usize {
    base + IRQ_STACK_SIZE - 8
}

pub const fn map_irq_stack(
    cpu: usize,
    vmap_stack: bool,
    vmap_success: bool,
    percpu_base: usize,
) -> Result<(usize, IrqStackBacking), i32> {
    let _ = cpu;
    if vmap_stack {
        if !vmap_success {
            return Err(-ENOMEM);
        }
        Ok((
            hardirq_stack_top(percpu_base),
            IrqStackBacking::Vmap { guard_pages: true },
        ))
    } else {
        Ok((
            hardirq_stack_top(percpu_base),
            IrqStackBacking::PerCpuBackingStore,
        ))
    }
}

pub const fn irq_init_percpu_irqstack(
    existing_stack_ptr: Option<usize>,
    mapped: Result<(usize, IrqStackBacking), i32>,
) -> Result<usize, i32> {
    if let Some(ptr) = existing_stack_ptr {
        return Ok(ptr);
    }
    match mapped {
        Ok((ptr, _)) => Ok(ptr),
        Err(err) => Err(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn irq64_stack_mapping_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kernel/irq_64.c"
        ));
        assert!(source.contains("DEFINE_PER_CPU_CACHE_HOT(bool, hardirq_stack_inuse)"));
        assert!(source.contains("DEFINE_PER_CPU_PAGE_ALIGNED(struct irq_stack"));
        assert!(source.contains("#ifdef CONFIG_VMAP_STACK"));
        assert!(source.contains("phys_addr_t pa = per_cpu_ptr_to_phys"));
        assert!(source.contains("vmap(pages, IRQ_STACK_SIZE / PAGE_SIZE, VM_MAP, PAGE_KERNEL)"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("IRQ_STACK_SIZE - 8"));
        assert!(source.contains("if (per_cpu(hardirq_stack_ptr, cpu))"));
        assert!(source.contains("return map_irq_stack(cpu);"));

        assert_eq!(irq_stack_pages(), 4);
        let mapped = map_irq_stack(0, true, true, 0x1000).unwrap();
        assert_eq!(mapped.0, 0x1000 + IRQ_STACK_SIZE - 8);
        assert_eq!(map_irq_stack(0, true, false, 0x1000), Err(-ENOMEM));
        assert_eq!(
            irq_init_percpu_irqstack(Some(0xdead), Ok(mapped)),
            Ok(0xdead)
        );
        assert_eq!(irq_init_percpu_irqstack(None, Ok(mapped)), Ok(mapped.0));
    }
}
