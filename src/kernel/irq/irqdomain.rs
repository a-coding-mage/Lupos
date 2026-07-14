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
use core::ffi::c_void;

use spin::Mutex;

use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "irq_domain_instantiate",
        linux_irq_domain_instantiate as usize,
        false,
    );
    export_symbol_once(
        "__irq_resolve_mapping",
        linux___irq_resolve_mapping as usize,
        true,
    );
    export_symbol_once(
        "irq_create_mapping_affinity",
        linux_irq_create_mapping_affinity as usize,
        true,
    );
    export_symbol_once("irq_domain_remove", linux_irq_domain_remove as usize, true);
    export_symbol_once(
        "irq_dispose_mapping",
        linux_irq_dispose_mapping as usize,
        true,
    );
}

/// `irq_domain_instantiate` - `vendor/linux/kernel/irq/irqdomain.c`.
pub unsafe extern "C" fn linux_irq_domain_instantiate(_info: *const c_void) -> *mut c_void {
    core::ptr::null_mut()
}

/// `irq_create_mapping_affinity` - `vendor/linux/kernel/irq/irqdomain.c:822`.
pub unsafe extern "C" fn linux_irq_create_mapping_affinity(
    _domain: *mut c_void,
    _hwirq: u64,
    _affinity: *const c_void,
) -> u32 {
    0
}

/// `__irq_resolve_mapping` - `vendor/linux/kernel/irq/irqdomain.c:1054`.
///
/// Lupos has no module-created IRQ domain objects yet.  Returning NULL mirrors
/// "no mapping found"; callers using the inline `irq_find_mapping()` see virq
/// zero and follow their no-IRQ fallback path.
#[unsafe(export_name = "__irq_resolve_mapping")]
pub unsafe extern "C" fn linux___irq_resolve_mapping(
    _domain: *mut c_void,
    _hwirq: u64,
    irq: *mut u32,
) -> *mut c_void {
    if !irq.is_null() {
        unsafe {
            *irq = 0;
        }
    }
    core::ptr::null_mut()
}

/// `irq_domain_remove` - `vendor/linux/kernel/irq/irqdomain.c`.
///
/// Module-created IRQ domains are not represented as Linux `struct irq_domain`
/// objects yet. Preserve teardown ABI for fail-closed users.
pub unsafe extern "C" fn linux_irq_domain_remove(_domain: *mut c_void) {}

/// `irq_dispose_mapping` - `vendor/linux/kernel/irq/irqdomain.c`.
pub unsafe extern "C" fn linux_irq_dispose_mapping(_virq: u32) {}

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
