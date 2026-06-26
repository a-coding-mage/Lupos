//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/um/sysrq_64.c
//! test-origin: linux:vendor/linux/arch/x86/um/sysrq_64.c
//! UML x86-64 register dump format.

pub const TASK_LINE: &str = "Pid: %d, comm: %.20s %s %s";

pub const REGISTER_LINES: &[&str] = &[
    "RIP: %04lx:%pS",
    "RSP: %016lx  EFLAGS: %08lx",
    "RAX: %016lx RBX: %016lx RCX: %016lx",
    "RDX: %016lx RSI: %016lx RDI: %016lx",
    "RBP: %016lx R08: %016lx R09: %016lx",
    "R10: %016lx R11: %016lx R12: %016lx",
    "R13: %016lx R14: %016lx R15: %016lx",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uml_sysrq_64_show_regs_format_matches_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/um/sysrq_64.c"
        ));
        assert!(source.contains("#include <asm/current.h>"));
        assert!(source.contains("#include <asm/ptrace.h>"));
        assert!(source.contains("void show_regs(struct pt_regs *regs)"));
        assert!(source.contains("print_modules();"));
        assert!(source.contains("task_pid_nr(current)"));
        assert!(source.contains("init_utsname()->release"));
        assert!(source.contains(TASK_LINE));
        assert!(source.contains("PT_REGS_CS(regs) & 0xffff"));
        for line in REGISTER_LINES {
            assert!(source.contains(line));
        }
        assert!(source.contains("PT_REGS_R8(regs)"));
        assert!(source.contains("PT_REGS_R15(regs)"));
    }
}
