//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/um/sysrq_32.c
//! test-origin: linux:vendor/linux/arch/x86/um/sysrq_32.c
//! UML x86 32-bit register dump format.

pub const REGISTER_LINES: &[&str] = &[
    "EIP: %04lx:[<%08lx>] CPU: %d %s",
    "ESP: %04lx:%08lx",
    "EFLAGS: %08lx",
    "EAX: %08lx EBX: %08lx ECX: %08lx EDX: %08lx",
    "ESI: %08lx EDI: %08lx EBP: %08lx",
    "DS: %04lx ES: %04lx",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uml_sysrq_32_show_regs_format_matches_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/um/sysrq_32.c"
        ));
        assert!(source.contains("#include <asm/ptrace.h>"));
        assert!(source.contains("void show_regs(struct pt_regs *regs)"));
        for line in REGISTER_LINES {
            assert!(source.contains(line));
        }
        assert!(source.contains("PT_REGS_CS(regs) & 3"));
        assert!(source.contains("print_tainted()"));
    }
}
