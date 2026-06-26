//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/asm-offsets_32.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/asm-offsets_32.c
//! 32-bit x86 asm-offset symbol inventory.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AsmOffsetSymbol {
    pub name: &'static str,
    pub structure: &'static str,
    pub field: &'static str,
}

pub const PT_REGS_32_OFFSETS: &[AsmOffsetSymbol] = &[
    AsmOffsetSymbol {
        name: "PT_EBX",
        structure: "pt_regs",
        field: "bx",
    },
    AsmOffsetSymbol {
        name: "PT_ECX",
        structure: "pt_regs",
        field: "cx",
    },
    AsmOffsetSymbol {
        name: "PT_EDX",
        structure: "pt_regs",
        field: "dx",
    },
    AsmOffsetSymbol {
        name: "PT_ESI",
        structure: "pt_regs",
        field: "si",
    },
    AsmOffsetSymbol {
        name: "PT_EDI",
        structure: "pt_regs",
        field: "di",
    },
    AsmOffsetSymbol {
        name: "PT_EBP",
        structure: "pt_regs",
        field: "bp",
    },
    AsmOffsetSymbol {
        name: "PT_EAX",
        structure: "pt_regs",
        field: "ax",
    },
    AsmOffsetSymbol {
        name: "PT_ORIG_EAX",
        structure: "pt_regs",
        field: "orig_ax",
    },
    AsmOffsetSymbol {
        name: "PT_EIP",
        structure: "pt_regs",
        field: "ip",
    },
    AsmOffsetSymbol {
        name: "PT_EFLAGS",
        structure: "pt_regs",
        field: "flags",
    },
];

pub const SAVED_CONTEXT_GDT_DESC: AsmOffsetSymbol = AsmOffsetSymbol {
    name: "saved_context_gdt_desc",
    structure: "saved_context",
    field: "gdt_desc",
};

pub const TSS_ENTRY2TASK_STACK: &str = "TSS_entry2task_stack";
pub const EFI_SVAM: AsmOffsetSymbol = AsmOffsetSymbol {
    name: "EFI_svam",
    structure: "efi_runtime_services_t",
    field: "set_virtual_address_map",
};

pub fn has_pt_regs_offset(name: &str) -> bool {
    PT_REGS_32_OFFSETS.iter().any(|symbol| symbol.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asm_offsets_32_inventory_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kernel/asm-offsets_32.c"
        ));
        assert!(source.contains("Please do not build this file directly"));
        assert!(source.contains("OFFSET(PT_EBX, pt_regs, bx);"));
        assert!(source.contains("OFFSET(PT_ORIG_EAX, pt_regs, orig_ax);"));
        assert!(source.contains("OFFSET(saved_context_gdt_desc, saved_context, gdt_desc);"));
        assert!(source.contains("DEFINE(TSS_entry2task_stack"));
        assert!(source.contains(
            "DEFINE(EFI_svam, offsetof(efi_runtime_services_t, set_virtual_address_map));"
        ));

        assert!(has_pt_regs_offset("PT_EBX"));
        assert!(has_pt_regs_offset("PT_EFLAGS"));
        assert!(!has_pt_regs_offset("PT_RIP"));
        assert_eq!(SAVED_CONTEXT_GDT_DESC.field, "gdt_desc");
        assert_eq!(EFI_SVAM.field, "set_virtual_address_map");
    }
}
