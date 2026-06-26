//! linux-parity: complete
//! linux-source: vendor/linux/lib/irq_regs.c
//! test-origin: linux:vendor/linux/lib/irq_regs.c
//! Saved per-CPU IRQ register pointer export.

pub const LINUX_SOURCE: &str = "vendor/linux/lib/irq_regs.c";
pub const PER_CPU_SYMBOL: &str = "__irq_regs";

pub const fn exports_irq_regs(arch_has_own_irq_regs: bool) -> bool {
    !arch_has_own_irq_regs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn irq_regs_export_is_arch_gated() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/irq_regs.c"
        ));
        assert!(source.contains("#ifndef ARCH_HAS_OWN_IRQ_REGS"));
        assert!(source.contains("DEFINE_PER_CPU(struct pt_regs *, __irq_regs);"));
        assert!(source.contains("EXPORT_PER_CPU_SYMBOL(__irq_regs);"));
        assert!(exports_irq_regs(false));
        assert!(!exports_irq_regs(true));
    }
}
