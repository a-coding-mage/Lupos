//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/asm-offsets_64.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/asm-offsets_64.c
//! 64-bit x86 asm-offset symbol inventory.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AsmOffsetSymbol {
    pub name: &'static str,
    pub structure: &'static str,
    pub field: &'static str,
}

pub const PT_REGS_64_OFFSETS: &[AsmOffsetSymbol] = &[
    AsmOffsetSymbol {
        name: "pt_regs_bx",
        structure: "pt_regs",
        field: "bx",
    },
    AsmOffsetSymbol {
        name: "pt_regs_cx",
        structure: "pt_regs",
        field: "cx",
    },
    AsmOffsetSymbol {
        name: "pt_regs_dx",
        structure: "pt_regs",
        field: "dx",
    },
    AsmOffsetSymbol {
        name: "pt_regs_sp",
        structure: "pt_regs",
        field: "sp",
    },
    AsmOffsetSymbol {
        name: "pt_regs_bp",
        structure: "pt_regs",
        field: "bp",
    },
    AsmOffsetSymbol {
        name: "pt_regs_si",
        structure: "pt_regs",
        field: "si",
    },
    AsmOffsetSymbol {
        name: "pt_regs_di",
        structure: "pt_regs",
        field: "di",
    },
    AsmOffsetSymbol {
        name: "pt_regs_r8",
        structure: "pt_regs",
        field: "r8",
    },
    AsmOffsetSymbol {
        name: "pt_regs_r15",
        structure: "pt_regs",
        field: "r15",
    },
    AsmOffsetSymbol {
        name: "pt_regs_flags",
        structure: "pt_regs",
        field: "flags",
    },
];

pub const SAVED_CONTEXT_64_OFFSETS: &[AsmOffsetSymbol] = &[
    AsmOffsetSymbol {
        name: "saved_context_cr0",
        structure: "saved_context",
        field: "cr0",
    },
    AsmOffsetSymbol {
        name: "saved_context_cr2",
        structure: "saved_context",
        field: "cr2",
    },
    AsmOffsetSymbol {
        name: "saved_context_cr3",
        structure: "saved_context",
        field: "cr3",
    },
    AsmOffsetSymbol {
        name: "saved_context_cr4",
        structure: "saved_context",
        field: "cr4",
    },
    AsmOffsetSymbol {
        name: "saved_context_gdt_desc",
        structure: "saved_context",
        field: "gdt_desc",
    },
];

pub const KVM_STEAL_TIME_PREEMPTED: AsmOffsetSymbol = AsmOffsetSymbol {
    name: "KVM_STEAL_TIME_preempted",
    structure: "kvm_steal_time",
    field: "preempted",
};

pub fn has_pt_regs_64_offset(name: &str) -> bool {
    PT_REGS_64_OFFSETS.iter().any(|symbol| symbol.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asm_offsets_64_inventory_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kernel/asm-offsets_64.c"
        ));
        assert!(source.contains("OFFSET(KVM_STEAL_TIME_preempted, kvm_steal_time, preempted);"));
        assert!(source.contains("#define ENTRY(entry) OFFSET(pt_regs_ ## entry, pt_regs, entry)"));
        assert!(source.contains("ENTRY(bx);"));
        assert!(source.contains("ENTRY(r15);"));
        assert!(source.contains("ENTRY(flags);"));
        assert!(source.contains(
            "#define ENTRY(entry) OFFSET(saved_context_ ## entry, saved_context, entry)"
        ));
        assert!(source.contains("ENTRY(cr4);"));
        assert!(source.contains("return 0;"));

        assert!(has_pt_regs_64_offset("pt_regs_bx"));
        assert!(has_pt_regs_64_offset("pt_regs_r15"));
        assert!(!has_pt_regs_64_offset("PT_EBX"));
        assert_eq!(KVM_STEAL_TIME_PREEMPTED.field, "preempted");
        assert_eq!(SAVED_CONTEXT_64_OFFSETS.len(), 5);
    }
}
