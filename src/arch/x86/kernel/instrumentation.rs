//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! x86 tracing, patching, module, probe, and BPF-JIT gates.
//!
//! Linux uses arch code to patch alternatives/static calls, write ftrace and
//! kprobe trampolines, relocate modules, unwind stacks, and JIT BPF programs.
//! Lupos keeps generic trace/BPF/module state elsewhere. Runtime text users
//! share the W^X-safe x86 text-poke backend; facilities without a production
//! execution path (currently uprobes and the BPF JIT) remain fail-closed.

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

pub const fn arch_instrumentation_enabled(kind: ArchInstrumentation) -> bool {
    matches!(
        kind,
        ArchInstrumentation::Ftrace
            | ArchInstrumentation::Kprobe
            | ArchInstrumentation::JumpLabel
            | ArchInstrumentation::StaticCall
            | ArchInstrumentation::ModuleRelocation
    )
}

pub const fn arch_instrumentation_errno(kind: ArchInstrumentation) -> i32 {
    if arch_instrumentation_enabled(kind) {
        0
    } else {
        EOPNOTSUPP
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implemented_text_patchers_are_enabled_and_unimplemented_jit_is_closed() {
        assert!(arch_instrumentation_enabled(ArchInstrumentation::Ftrace));
        assert!(arch_instrumentation_enabled(ArchInstrumentation::Kprobe));
        assert!(arch_instrumentation_enabled(ArchInstrumentation::JumpLabel));
        assert!(arch_instrumentation_enabled(
            ArchInstrumentation::StaticCall
        ));
        assert_eq!(arch_instrumentation_errno(ArchInstrumentation::Ftrace), 0);
        assert!(!arch_instrumentation_enabled(ArchInstrumentation::BpfJit));
        assert!(!arch_instrumentation_enabled(ArchInstrumentation::Rethook));
        assert_eq!(
            arch_instrumentation_errno(ArchInstrumentation::BpfJit),
            EOPNOTSUPP
        );
    }
}
