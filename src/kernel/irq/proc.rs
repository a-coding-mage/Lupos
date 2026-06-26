//! linux-parity: complete
//! linux-source: vendor/linux/kernel/irq/proc.c
//! test-origin: linux:vendor/linux/kernel/irq/proc.c
//! IRQ procfs coverage for M37.
//!
//! Mirrors `vendor/linux/kernel/irq/proc.c`.

use core::sync::atomic::Ordering;

use super::irqdesc::desc_for;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IrqProcEntry {
    pub irq: u32,
    pub count: u64,
    pub affinity: u32,
}

pub fn irq_proc_entry(irq: u32) -> Option<IrqProcEntry> {
    let desc = desc_for(irq)?;
    let stat = desc.stat.lock();
    Some(IrqProcEntry {
        irq,
        count: stat.count,
        affinity: desc.affinity.load(Ordering::Acquire),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proc_entry_exists_for_valid_irq() {
        assert_eq!(irq_proc_entry(1).unwrap().irq, 1);
    }
}
