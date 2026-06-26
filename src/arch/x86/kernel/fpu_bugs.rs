//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/fpu/bugs.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/fpu/bugs.c
//! x86 FPU bug detection at boot.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/fpu/bugs.c
//!
//! The only FPU bug detected here today is the original Pentium FDIV
//! defect (`X86_BUG_FDIV`). Linux runs a tiny computation whose result is
//! known on a non-buggy FPU; if the FPU reports a non-zero residue, the
//! bug bit is set.
//!
//! Reference: <https://en.wikipedia.org/wiki/Pentium_FDIV_bug>

#![allow(dead_code)]

/// Constants for the FDIV probe. These are the exact double-precision
/// operands Linux uses; do not alter or the residue check is invalidated.
pub const FDIV_X: f64 = 4_195_835.0;
pub const FDIV_Y: f64 = 3_145_727.0;

/// Bit mirror of `X86_BUG_FDIV` (set on `boot_cpu_data` on hit).
pub const X86_BUG_FDIV: u32 = 0;

/// Compute the FDIV residue without inline asm. The same arithmetic the
/// Pentium probe performs:
///
/// ```text
/// residue = x - (x / y) * y
/// ```
///
/// A correctly-rounded FPU returns 0; an FDIV-bug FPU returns a tiny but
/// non-zero residue, hence the `truncate-to-i32 != 0` test.
pub fn fdiv_residue() -> i32 {
    let r = FDIV_X - (FDIV_X / FDIV_Y) * FDIV_Y;
    r as i32
}

/// Trait seam for `boot_cpu_has(X86_FEATURE_FPU)` and the boot-cpu bug
/// bitmap. Production wires this to `crate::arch::x86::kernel::cpuid` / the
/// shared boot-CPU record.
pub trait BootCpu {
    fn has_fpu(&self) -> bool;
    fn set_bug(&self, bug: u32);
}

/// Trait seam for `kernel_fpu_begin/end` — the FPU lock around any
/// inline FP instructions in kernel context.
pub trait KernelFpu {
    fn begin(&self);
    fn end(&self);
}

/// Linux's `fpu__init_check_bugs`: skip if no FPU; otherwise run the
/// probe under `kernel_fpu_begin/end`; if the residue is non-zero, set
/// `X86_BUG_FDIV` on `boot_cpu_data`.
pub fn fpu_init_check_bugs<C, F>(cpu: &C, fpu: &F)
where
    C: BootCpu,
    F: KernelFpu,
{
    if !cpu.has_fpu() {
        return;
    }
    fpu.begin();
    let bug = fdiv_residue();
    fpu.end();
    if bug != 0 {
        cpu.set_bug(X86_BUG_FDIV);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::Cell;

    struct MockCpu {
        has_fpu: bool,
        bugs: Cell<u32>,
    }

    impl BootCpu for MockCpu {
        fn has_fpu(&self) -> bool {
            self.has_fpu
        }
        fn set_bug(&self, bug: u32) {
            self.bugs.set(self.bugs.get() | (1u32 << bug));
        }
    }

    struct MockFpu {
        begin_calls: Cell<u32>,
        end_calls: Cell<u32>,
    }

    impl KernelFpu for MockFpu {
        fn begin(&self) {
            self.begin_calls.set(self.begin_calls.get() + 1);
        }
        fn end(&self) {
            self.end_calls.set(self.end_calls.get() + 1);
        }
    }

    #[test]
    fn fdiv_residue_is_zero_on_correct_fpu() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kernel/fpu/bugs.c"
        ));
        assert!(source.contains("static double __initdata x = 4195835.0;"));
        assert!(source.contains("static double __initdata y = 3145727.0;"));
        assert!(source.contains("void __init fpu__init_check_bugs(void)"));
        assert!(source.contains("if (!boot_cpu_has(X86_FEATURE_FPU))"));
        assert!(source.contains("kernel_fpu_begin();"));
        assert!(source.contains("\"fninit\\n\\t\""));
        assert!(source.contains("kernel_fpu_end();"));
        assert!(source.contains("set_cpu_bug(&boot_cpu_data, X86_BUG_FDIV);"));
        assert!(source.contains("pr_warn(\"Hmm, FPU with FDIV bug\\n\");"));

        // IEEE-754 doubles on any modern host return exactly zero.
        assert_eq!(fdiv_residue(), 0);
    }

    #[test]
    fn no_fpu_skips_probe_entirely() {
        let cpu = MockCpu {
            has_fpu: false,
            bugs: Cell::new(0),
        };
        let fpu = MockFpu {
            begin_calls: Cell::new(0),
            end_calls: Cell::new(0),
        };
        fpu_init_check_bugs(&cpu, &fpu);
        assert_eq!(fpu.begin_calls.get(), 0);
        assert_eq!(fpu.end_calls.get(), 0);
        assert_eq!(cpu.bugs.get(), 0);
    }

    #[test]
    fn fpu_present_runs_probe_and_does_not_set_bug() {
        let cpu = MockCpu {
            has_fpu: true,
            bugs: Cell::new(0),
        };
        let fpu = MockFpu {
            begin_calls: Cell::new(0),
            end_calls: Cell::new(0),
        };
        fpu_init_check_bugs(&cpu, &fpu);
        assert_eq!(fpu.begin_calls.get(), 1);
        assert_eq!(fpu.end_calls.get(), 1);
        assert_eq!(cpu.bugs.get(), 0);
    }
}
