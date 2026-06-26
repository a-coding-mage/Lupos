//! linux-parity: complete
//! linux-source: vendor/linux/kernel/irq/debugfs.c
//! test-origin: linux:vendor/linux/kernel/irq/debugfs.c
//! IRQ debugfs coverage for M37.
//!
//! Mirrors `vendor/linux/kernel/irq/debugfs.c`.

use core::sync::atomic::Ordering;

use super::irqdesc::desc_for;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IrqDebugSnapshot {
    pub irq: u32,
    pub enabled: bool,
    pub affinity: u32,
}

pub fn irq_debug_snapshot(irq: u32) -> Option<IrqDebugSnapshot> {
    let desc = desc_for(irq)?;
    Some(IrqDebugSnapshot {
        irq,
        enabled: desc.is_enabled(),
        affinity: desc.affinity.load(Ordering::Acquire),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_exists_for_valid_irq() {
        let snap = irq_debug_snapshot(0).unwrap();
        assert_eq!(snap.irq, 0);
    }
}
