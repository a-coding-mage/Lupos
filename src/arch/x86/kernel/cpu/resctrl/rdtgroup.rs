//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kernel/cpu/resctrl/rdtgroup.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/resctrl/rdtgroup.c
//! resctrl filesystem group lifecycle.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/resctrl/rdtgroup.c

// rdtgroups live under /sys/fs/resctrl. The root group always exists,
// child groups can be control-only, monitor-only, or both. We model the
// group kind and the basic create/remove invariants without owning the
// filesystem.

use crate::include::uapi::errno::EINVAL;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RdtGroupKind {
    Control,
    Monitor,
    ControlAndMonitor,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RdtGroup {
    pub name: alloc::string::String,
    pub kind: RdtGroupKind,
    pub closid: Option<u32>,
    pub rmid: Option<u32>,
}

extern crate alloc;

pub fn validate_name(name: &str) -> Result<(), i32> {
    if name.is_empty() || name == "." || name == ".." {
        return Err(EINVAL);
    }
    if name.contains('/') || name.contains('\0') {
        return Err(EINVAL);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_path_separator_and_special_names() {
        assert_eq!(validate_name(""), Err(EINVAL));
        assert_eq!(validate_name("a/b"), Err(EINVAL));
        assert_eq!(validate_name("."), Err(EINVAL));
        assert!(validate_name("workload").is_ok());
    }
}
