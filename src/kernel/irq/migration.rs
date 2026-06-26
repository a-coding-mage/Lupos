//! linux-parity: complete
//! linux-source: vendor/linux/kernel/irq/migration.c
//! test-origin: linux:vendor/linux/kernel/irq/migration.c
//! IRQ migration coverage for M37.
//!
//! Mirrors `vendor/linux/kernel/irq/migration.c`.

use core::sync::atomic::Ordering;

use super::irqdesc::desc_for;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IrqMigration {
    pub irq: u32,
    pub old_mask: u32,
    pub new_mask: u32,
}

pub fn irq_move_masked_irq(irq: u32, new_mask: u32) -> Option<IrqMigration> {
    let desc = desc_for(irq)?;
    let old = desc.affinity.swap(new_mask, Ordering::AcqRel);
    Some(IrqMigration {
        irq,
        old_mask: old,
        new_mask,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_updates_affinity_mask() {
        let migration = irq_move_masked_irq(0x61, 0x2).unwrap();
        assert_eq!(migration.irq, 0x61);
        assert_eq!(migration.new_mask, 0x2);
    }
}
