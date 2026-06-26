//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/hyperv/mshv-asm-offsets.c
//! test-origin: linux:vendor/linux/arch/x86/hyperv/mshv-asm-offsets.c
//! Microsoft Hyper-V VTL context assembly-offset symbol generator.

pub const MSHV_VTL_CPU_CONTEXT_OFFSETS: &[&str] = &[
    "MSHV_VTL_CPU_CONTEXT_rax",
    "MSHV_VTL_CPU_CONTEXT_rcx",
    "MSHV_VTL_CPU_CONTEXT_rdx",
    "MSHV_VTL_CPU_CONTEXT_rbx",
    "MSHV_VTL_CPU_CONTEXT_rbp",
    "MSHV_VTL_CPU_CONTEXT_rsi",
    "MSHV_VTL_CPU_CONTEXT_rdi",
    "MSHV_VTL_CPU_CONTEXT_r8",
    "MSHV_VTL_CPU_CONTEXT_r9",
    "MSHV_VTL_CPU_CONTEXT_r10",
    "MSHV_VTL_CPU_CONTEXT_r11",
    "MSHV_VTL_CPU_CONTEXT_r12",
    "MSHV_VTL_CPU_CONTEXT_r13",
    "MSHV_VTL_CPU_CONTEXT_r14",
    "MSHV_VTL_CPU_CONTEXT_r15",
    "MSHV_VTL_CPU_CONTEXT_cr2",
];

pub const fn mshv_asm_offsets(vtl_mode: bool) -> &'static [&'static str] {
    if vtl_mode {
        MSHV_VTL_CPU_CONTEXT_OFFSETS
    } else {
        &[]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mshv_offsets_are_emitted_only_for_vtl_mode() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/hyperv/mshv-asm-offsets.c"
        ));
        assert!(source.contains("if (IS_ENABLED(CONFIG_HYPERV_VTL_MODE))"));
        assert!(source.contains("OFFSET(MSHV_VTL_CPU_CONTEXT_rax"));
        assert!(source.contains("OFFSET(MSHV_VTL_CPU_CONTEXT_cr2"));

        assert!(mshv_asm_offsets(false).is_empty());
        assert_eq!(mshv_asm_offsets(true).len(), 16);
        assert_eq!(mshv_asm_offsets(true)[0], "MSHV_VTL_CPU_CONTEXT_rax");
        assert_eq!(mshv_asm_offsets(true)[15], "MSHV_VTL_CPU_CONTEXT_cr2");
    }
}
