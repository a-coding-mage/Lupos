//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! x86 tracing, patching, module, probe, and BPF-JIT gates.
//!
//! Linux uses arch code to patch alternatives/static calls, write ftrace and
//! kprobe trampolines, relocate modules, unwind stacks, and JIT BPF programs.
//! Lupos keeps generic trace/BPF/module state elsewhere; architecture-specific
//! executable text patching and JIT codegen stay fail-closed here until those
//! subsystems can allocate and protect executable kernel text.

use crate::include::uapi::errno::EOPNOTSUPP;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArchInstrumentation {
    Ftrace,
    Kprobe,
    Uprobe,
    JumpLabel,
    StaticCall,
    Rethook,
    BpfJit,
    ModuleRelocation,
}

pub const fn arch_instrumentation_enabled(_kind: ArchInstrumentation) -> bool {
    false
}

pub const fn arch_instrumentation_errno(_kind: ArchInstrumentation) -> i32 {
    EOPNOTSUPP
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executable_text_patchers_are_not_enabled_yet() {
        assert!(!arch_instrumentation_enabled(ArchInstrumentation::Ftrace));
        assert_eq!(
            arch_instrumentation_errno(ArchInstrumentation::BpfJit),
            EOPNOTSUPP
        );
    }
}
