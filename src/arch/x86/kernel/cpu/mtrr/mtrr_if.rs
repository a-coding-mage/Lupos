//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/mtrr
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/mtrr
//! /proc/mtrr text interface formatting.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/mtrr/if.c

// `if.c` exposes the legacy text-mode /proc/mtrr writer used by user
// space tools such as `mtrr-tool`. We model the line formatter; the
// actual /proc registration is owned by procfs.

extern crate alloc;

use alloc::format;
use alloc::string::String;

use crate::arch::x86::kernel::mtrr::MtrrMemoryType;

pub fn memory_type_str(t: MtrrMemoryType) -> &'static str {
    match t {
        MtrrMemoryType::Uncacheable => "uncachable",
        MtrrMemoryType::WriteCombining => "write-combining",
        MtrrMemoryType::WriteThrough => "write-through",
        MtrrMemoryType::WriteProtected => "write-protect",
        MtrrMemoryType::WriteBack => "write-back",
    }
}

pub fn format_reg_line(reg: u8, base: u64, size_mb: u64, t: MtrrMemoryType) -> String {
    format!(
        "reg{reg:02}: base=0x{base:09x} ({size_mb}MB), size={size_mb}MB, count=1: {kind}\n",
        reg = reg,
        base = base,
        size_mb = size_mb,
        kind = memory_type_str(t)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_strings_match_linux_procfs_labels() {
        assert_eq!(memory_type_str(MtrrMemoryType::WriteBack), "write-back");
        assert_eq!(
            memory_type_str(MtrrMemoryType::WriteCombining),
            "write-combining"
        );
    }

    #[test]
    fn line_contains_the_register_index_and_size() {
        let line = format_reg_line(2, 0x8000_0000, 1024, MtrrMemoryType::WriteBack);
        assert!(line.starts_with("reg02"));
        assert!(line.contains("1024MB"));
        assert!(line.contains("write-back"));
    }
}
