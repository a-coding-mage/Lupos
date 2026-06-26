//! linux-parity: complete
//! linux-source: vendor/linux/kernel/cfi.c
//! test-origin: linux:vendor/linux/kernel/cfi.c
//! Generic Clang CFI failure reporting and trap lookup helpers.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BugTrapType {
    Warn,
    Bug,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CfiFailureReport {
    pub addr: usize,
    pub target: Option<usize>,
    pub expected_type: u32,
    pub trap_type: BugTrapType,
}

pub const fn report_cfi_failure(
    cfi_warn: bool,
    addr: usize,
    target: Option<usize>,
    expected_type: u32,
) -> CfiFailureReport {
    CfiFailureReport {
        addr,
        target,
        expected_type,
        trap_type: if cfi_warn {
            BugTrapType::Warn
        } else {
            BugTrapType::Bug
        },
    }
}

pub const fn trap_address(entry_addr: isize, displacement: i32) -> usize {
    (entry_addr + displacement as isize) as usize
}

pub fn is_trap(addr: usize, trap_entries: &[(usize, i32)]) -> bool {
    trap_entries
        .iter()
        .any(|(entry_addr, displacement)| trap_address(*entry_addr as isize, *displacement) == addr)
}

pub fn module_cfi_finalize(sections: &[(&str, usize, usize)]) -> Option<(usize, usize)> {
    sections
        .iter()
        .find(|(name, _, _)| *name == "__kcfi_traps")
        .map(|(_, addr, size)| (*addr, addr.saturating_add(*size)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_cfi_failure_and_trap_lookup_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/cfi.c"
        ));
        assert!(
            source.contains("bool cfi_warn __ro_after_init = IS_ENABLED(CONFIG_CFI_PERMISSIVE);")
        );
        assert!(source.contains("enum bug_trap_type report_cfi_failure"));
        assert!(source.contains("CFI failure at %pS (target: %pS; expected type: 0x%08x)"));
        assert!(source.contains("CFI failure at %pS (no target information)"));
        assert!(source.contains("return BUG_TRAP_TYPE_WARN;"));
        assert!(source.contains("return BUG_TRAP_TYPE_BUG;"));
        assert!(source.contains("DEFINE_CFI_TYPE(cfi_bpf_hash, __bpf_prog_runX);"));
        assert!(source.contains("DEFINE_CFI_TYPE(cfi_bpf_subprog_hash, __bpf_callback_fn);"));
        assert!(source.contains("static inline unsigned long trap_address(s32 *p)"));
        assert!(source.contains("return (unsigned long)((long)p + (long)*p);"));
        assert!(source.contains("if (trap_address(p) == addr)"));
        assert!(source.contains("strcmp(secstrings + sechdrs[i].sh_name, \"__kcfi_traps\")"));
        assert!(source.contains("mod->kcfi_traps_end"));
        assert!(source.contains("bool is_cfi_trap(unsigned long addr)"));

        assert_eq!(
            report_cfi_failure(true, 0x1000, Some(0x2000), 0x55).trap_type,
            BugTrapType::Warn
        );
        assert_eq!(
            report_cfi_failure(false, 0x1000, None, 0x55).trap_type,
            BugTrapType::Bug
        );
        assert_eq!(trap_address(0x1000, 4), 0x1004);
        assert!(is_trap(0x1004, &[(0x1000, 4)]));
        assert!(!is_trap(0x1008, &[(0x1000, 4)]));
        assert_eq!(
            module_cfi_finalize(&[(".text", 0x1000, 0x20), ("__kcfi_traps", 0x2000, 8)]),
            Some((0x2000, 0x2008))
        );
    }
}
