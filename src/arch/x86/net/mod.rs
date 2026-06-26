//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/net
//! test-origin: linux:vendor/linux/arch/x86/net
//! x86 networking accelerator hooks.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/net/bpf_jit_comp.c
//! - vendor/linux/arch/x86/net/bpf_jit_comp32.c

use crate::include::uapi::errno::EOPNOTSUPP;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BpfJitCaps {
    pub x86_64: bool,
    pub has_sse2: bool,
    pub has_retpoline: bool,
    pub constant_blinding: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BpfJitBackend {
    Interpreter,
    X86_64,
    X86_32,
}

pub const fn select_bpf_jit_backend(caps: BpfJitCaps) -> BpfJitBackend {
    if caps.x86_64 && caps.has_sse2 {
        BpfJitBackend::X86_64
    } else if !caps.x86_64 {
        BpfJitBackend::X86_32
    } else {
        BpfJitBackend::Interpreter
    }
}

pub const fn bpf_jit_requires_retpoline(caps: BpfJitCaps) -> Result<(), i32> {
    if caps.has_retpoline || !caps.constant_blinding {
        Ok(())
    } else {
        Err(EOPNOTSUPP)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x86_64_jit_requires_sse2_in_the_arch_backend() {
        assert_eq!(
            select_bpf_jit_backend(BpfJitCaps {
                x86_64: true,
                has_sse2: false,
                has_retpoline: true,
                constant_blinding: false,
            }),
            BpfJitBackend::Interpreter
        );
        assert_eq!(
            select_bpf_jit_backend(BpfJitCaps {
                x86_64: false,
                has_sse2: false,
                has_retpoline: true,
                constant_blinding: false,
            }),
            BpfJitBackend::X86_32
        );
    }
}
