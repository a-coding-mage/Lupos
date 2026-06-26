//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/debugfs.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/debugfs.c
//! Per-CPU debugfs entry registry (modeled).
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/debugfs.c

// Linux creates `/sys/kernel/debug/x86/cpus/N/` directories at boot. The
// per-CPU `tsc_offset`, `apic_id`, etc. files are owned by other modules
// but the registration shape lives here. We model the entry list so
// observability code can register/unregister without touching debugfs
// itself.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CpuDebugFile {
    pub name: String,
    pub value: u64,
}

#[derive(Default, Debug)]
pub struct CpuDebugRegistry {
    files: Vec<CpuDebugFile>,
}

impl CpuDebugRegistry {
    pub fn register(&mut self, file: CpuDebugFile) {
        self.files.push(file);
    }

    pub fn unregister(&mut self, name: &str) -> bool {
        let before = self.files.len();
        self.files.retain(|f| f.name != name);
        self.files.len() != before
    }

    pub fn lookup(&self, name: &str) -> Option<u64> {
        self.files.iter().find(|f| f.name == name).map(|f| f.value)
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_lookup_round_trip() {
        let mut reg = CpuDebugRegistry::default();
        reg.register(CpuDebugFile {
            name: String::from("apic_id"),
            value: 7,
        });
        assert_eq!(reg.lookup("apic_id"), Some(7));
        assert!(reg.unregister("apic_id"));
        assert_eq!(reg.lookup("apic_id"), None);
        assert!(!reg.unregister("missing"));
    }
}
