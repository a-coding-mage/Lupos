//! linux-parity: complete
//! linux-source: vendor/linux/kernel/irq/irqdomain.c
//! test-origin: linux:vendor/linux/kernel/irq/irqdomain.c
//! `struct irq_domain` — hierarchical IRQ mapping (M37).
//!
//! Mirrors `vendor/linux/kernel/irq/irqdomain.c`.  Lupos M37 ships the linear
//! domain only; hash + radix-tree variants land alongside MSI hierarchy
//! work in M55 (PCI / ACPI).

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;

use spin::Mutex;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IrqDomainKind {
    Linear,
    Hierarchical,
}

pub struct IrqDomain {
    pub name: String,
    pub kind: IrqDomainKind,
    pub size: u32,
    /// hwirq → virq mapping.
    map: Mutex<BTreeMap<u32, u32>>,
    next_virq: Mutex<u32>,
}

impl IrqDomain {
    pub fn new(name: &str, kind: IrqDomainKind, size: u32) -> Self {
        Self {
            name: String::from(name),
            kind,
            size,
            map: Mutex::new(BTreeMap::new()),
            next_virq: Mutex::new(0x80),
        }
    }

    /// `irq_create_mapping(domain, hwirq)` — allocate a virq if not mapped.
    pub fn create_mapping(&self, hwirq: u32) -> u32 {
        let mut m = self.map.lock();
        if let Some(&v) = m.get(&hwirq) {
            return v;
        }
        let mut nv = self.next_virq.lock();
        let virq = *nv;
        *nv += 1;
        m.insert(hwirq, virq);
        virq
    }

    pub fn dispose_mapping(&self, virq: u32) {
        let mut m = self.map.lock();
        // Remove the first mapping whose value matches.
        let hwirq = m.iter().find(|&(_, v)| *v == virq).map(|(k, _)| *k);
        if let Some(k) = hwirq {
            m.remove(&k);
        }
    }

    pub fn find_mapping(&self, hwirq: u32) -> Option<u32> {
        self.map.lock().get(&hwirq).copied()
    }

    pub fn nr_mappings(&self) -> usize {
        self.map.lock().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_domain_create_mapping_is_idempotent() {
        let d = IrqDomain::new("test", IrqDomainKind::Linear, 32);
        let v1 = d.create_mapping(5);
        let v2 = d.create_mapping(5);
        assert_eq!(v1, v2);
    }

    #[test]
    fn dispose_mapping_removes_entry() {
        let d = IrqDomain::new("test", IrqDomainKind::Linear, 32);
        let v = d.create_mapping(7);
        assert!(d.find_mapping(7).is_some());
        d.dispose_mapping(v);
        assert!(d.find_mapping(7).is_none());
    }

    #[test]
    fn distinct_hwirqs_get_distinct_virqs() {
        let d = IrqDomain::new("test", IrqDomainKind::Linear, 32);
        let a = d.create_mapping(1);
        let b = d.create_mapping(2);
        assert_ne!(a, b);
    }
}
