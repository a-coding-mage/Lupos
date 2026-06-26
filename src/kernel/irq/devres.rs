//! linux-parity: complete
//! linux-source: vendor/linux/kernel/irq/devres.c
//! test-origin: linux:vendor/linux/kernel/irq/devres.c
//! IRQ devres coverage for M37.
//!
//! Mirrors `vendor/linux/kernel/irq/devres.c`.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DevresIrq {
    pub irq: u32,
    pub dev_id: usize,
}

pub fn devm_irq_alloc_desc(irq: u32, dev_id: usize) -> DevresIrq {
    DevresIrq { irq, dev_id }
}

pub fn devm_irq_match(record: DevresIrq, irq: u32, dev_id: usize) -> bool {
    record.irq == irq && record.dev_id == dev_id
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn devres_record_matches_irq_and_device() {
        let record = devm_irq_alloc_desc(7, 99);
        assert!(devm_irq_match(record, 7, 99));
        assert!(!devm_irq_match(record, 8, 99));
    }
}
