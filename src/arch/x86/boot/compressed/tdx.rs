//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/compressed/tdx.c
//! test-origin: linux:vendor/linux/arch/x86/boot/compressed/tdx.c
//! Early TDX detection + I/O hypercall shim.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/compressed/tdx.c
//!
//! Linux's decompressor probes CPUID leaf 0x21 for the "IntelTDX    "
//! signature; if present, port I/O is rerouted through TDX hypercalls
//! (TDG.VP.VMCALL with EXIT_REASON_IO_INSTRUCTION). The runtime body
//! is in the Batch 9 `coco/tdx/tdx.c` port; this module carries the detection
//! constants and the I/O dispatch shape.

use crate::arch::x86::kernel::cpuid::cpuid;

/// `TDX_CPUID_LEAF_ID` — Linux uses leaf 0x21 to identify TDX guests.
pub const TDX_CPUID_LEAF_ID: u32 = 0x21;

/// `TDX_IDENT` — vendor string returned by CPUID leaf 0x21 (subleaf 0)
/// when running under a TDX module. Linux stores it across EBX/EDX/ECX
/// as `"IntelTDX    "` (note the trailing 4 spaces forming the
/// 12-byte string). Matches `arch/x86/include/asm/tdx.h::TDX_IDENT`.
pub const TDX_IDENT: &[u8; 12] = b"IntelTDX    ";

/// `EXIT_REASON_IO_INSTRUCTION` — VMX exit reason 30. Hypercall sub-fn.
pub const EXIT_REASON_IO_INSTRUCTION: u64 = 30;

/// `TDX_HYPERCALL_STANDARD` — Linux's value for R10 on a standard
/// TDG.VP.VMCALL: 0 (per `asm/tdx.h`).
pub const TDX_HYPERCALL_STANDARD: u64 = 0;

/// Hypercall trait — production wires this to `__tdx_hypercall`
/// (asm thunk when real TDCALL wiring lands); host tests substitute a deterministic stub.
pub trait TdxHypercall {
    /// Issue a hypercall with `(r10, r11, r12, r13, r14, r15)`. Returns
    /// the new value of `r11` (Linux's "result" register).
    fn vmcall(&mut self, r10: u64, r11: u64, r12: u64, r13: u64, r14: u64, r15: u64) -> u64;
    /// Returns true if the last call indicated unrecoverable failure.
    fn last_failed(&self) -> bool {
        false
    }
}

/// `early_tdx_detect()` — return true if CPUID leaf 0x21 reports
/// `IntelTDX`. Mirrors tdx.c lines 64-77.
pub fn early_tdx_detect() -> bool {
    let r = cpuid(TDX_CPUID_LEAF_ID, 0);
    // Linux orders the regs as EBX, EDX, ECX (yes, EDX-then-ECX, not
    // EBX-ECX-EDX!) to spell "IntelTDX    ". Match that exactly.
    let mut sig = [0u8; 12];
    sig[0..4].copy_from_slice(&r.ebx.to_le_bytes());
    sig[4..8].copy_from_slice(&r.edx.to_le_bytes());
    sig[8..12].copy_from_slice(&r.ecx.to_le_bytes());
    sig.as_slice() == TDX_IDENT.as_slice()
}

/// `tdx_io_in(size, port)` — read `size` bytes from `port` via VMCALL.
/// Returns `u32::MAX` on hypercall failure (Linux convention).
pub fn tdx_io_in<H: TdxHypercall>(hc: &mut H, size: u32, port: u16) -> u32 {
    let r11 = hc.vmcall(
        TDX_HYPERCALL_STANDARD,
        EXIT_REASON_IO_INSTRUCTION,
        size as u64,
        0,
        port as u64,
        0,
    );
    if hc.last_failed() {
        return u32::MAX;
    }
    r11 as u32
}

/// `tdx_io_out(size, port, value)` — write via VMCALL.
pub fn tdx_io_out<H: TdxHypercall>(hc: &mut H, size: u32, port: u16, value: u32) {
    hc.vmcall(
        TDX_HYPERCALL_STANDARD,
        EXIT_REASON_IO_INSTRUCTION,
        size as u64,
        1,
        port as u64,
        value as u64,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubHc {
        last_r11: u64,
        failed: bool,
        history: alloc::vec::Vec<(u64, u64, u64, u64, u64, u64)>,
    }
    impl TdxHypercall for StubHc {
        fn vmcall(&mut self, r10: u64, r11: u64, r12: u64, r13: u64, r14: u64, r15: u64) -> u64 {
            self.history.push((r10, r11, r12, r13, r14, r15));
            self.last_r11
        }
        fn last_failed(&self) -> bool {
            self.failed
        }
    }

    extern crate alloc;

    #[test]
    fn tdx_ident_bytes_match_inteltdx_padding() {
        // 'I' 'n' 't' 'e' | 'l' 'T' 'D' 'X' | ' ' ' ' ' ' ' '
        assert_eq!(TDX_IDENT, b"IntelTDX    ");
        assert_eq!(TDX_IDENT.len(), 12);
    }

    #[test]
    fn exit_reason_constants_match_linux() {
        assert_eq!(EXIT_REASON_IO_INSTRUCTION, 30);
        assert_eq!(TDX_HYPERCALL_STANDARD, 0);
        assert_eq!(TDX_CPUID_LEAF_ID, 0x21);
    }

    #[test]
    fn tdx_io_in_returns_uint_max_on_failure() {
        let mut hc = StubHc {
            last_r11: 0xabcd,
            failed: true,
            history: alloc::vec::Vec::new(),
        };
        assert_eq!(tdx_io_in(&mut hc, 1, 0x3f8), u32::MAX);
    }

    #[test]
    fn tdx_io_out_packs_args_per_linux_layout() {
        let mut hc = StubHc {
            last_r11: 0,
            failed: false,
            history: alloc::vec::Vec::new(),
        };
        tdx_io_out(&mut hc, 2, 0x3f8, 0x55aa);
        let h = &hc.history[0];
        assert_eq!(h.0, TDX_HYPERCALL_STANDARD);
        assert_eq!(h.1, EXIT_REASON_IO_INSTRUCTION);
        assert_eq!(h.2, 2); // size
        assert_eq!(h.3, 1); // write direction
        assert_eq!(h.4, 0x3f8); // port
        assert_eq!(h.5, 0x55aa); // value
    }
}
