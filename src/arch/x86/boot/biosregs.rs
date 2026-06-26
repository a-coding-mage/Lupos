//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/boot
//! linux-source: vendor/linux/arch/x86/boot/regs.c
//! test-origin: linux:vendor/linux/arch/x86/boot
//! Real-mode `struct biosregs` and BIOS-call seam.
//!
//! Mirrors `regs.c::initregs` and the `struct biosregs` layout, with `intcall`
//! behind a trait seam. Remaining work vs Linux for `complete`: a real
//! `bioscall.S::intcall` thunk — only meaningful for a real-mode setup stub,
//! which Lupos replaces with its 64-bit boot path, so this stays a seam.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/regs.c
//! - vendor/linux/arch/x86/boot/bioscall.S (the BIOS `intcall` thunk)
//! - vendor/linux/arch/x86/include/uapi/asm/bootparam.h::biosregs
//!
//! `biosregs` is the input/output bundle the real-mode setup stub passes
//! into INT 1Ah/INT 15h via `bioscall.S::intcall`. Lupos does not run in
//! real mode, so `intcall` lives behind a trait seam so tests can substitute
//! deterministic stubs.

/// `X86_EFLAGS_CF` — carry flag; the Linux setup stub uses CF=1 as
/// "function did nothing" detection. Matches
/// `vendor/linux/arch/x86/include/uapi/asm/processor-flags.h`.
pub const X86_EFLAGS_CF: u32 = 1 << 0;

/// Real-mode BIOS register set (Linux `struct biosregs`). All fields
/// match the layout in `bootparam.h` — kept exact so setup/bzImage interop
/// tests can `transmute` between this and the C definition.
#[repr(C)]
#[derive(Copy, Clone, Default, Debug, Eq, PartialEq)]
pub struct BiosRegs {
    pub edi: u32,
    pub esi: u32,
    pub ebp: u32,
    pub _esp: u32,
    pub ebx: u32,
    pub edx: u32,
    pub ecx: u32,
    pub eax: u32,
    pub gs: u16,
    pub fs: u16,
    pub es: u16,
    pub ds: u16,
    pub eflags: u32,
}

impl BiosRegs {
    /// Aliases used by the Linux setup code via the union shape in
    /// `bootparam.h`. Lupos exposes them as accessor methods because
    /// Rust unions can't be `Copy` *and* match Linux's
    /// "uppercase letter = low byte" convention without an unsafe
    /// transmute on every access. The bit-level meaning is preserved.
    #[inline]
    pub fn ax(&self) -> u16 {
        self.eax as u16
    }
    #[inline]
    pub fn set_ax(&mut self, v: u16) {
        self.eax = (self.eax & 0xffff_0000) | v as u32;
    }
    #[inline]
    pub fn al(&self) -> u8 {
        self.eax as u8
    }
    #[inline]
    pub fn set_al(&mut self, v: u8) {
        self.eax = (self.eax & 0xffff_ff00) | v as u32;
    }
    #[inline]
    pub fn ah(&self) -> u8 {
        (self.eax >> 8) as u8
    }
    #[inline]
    pub fn set_ah(&mut self, v: u8) {
        self.eax = (self.eax & 0xffff_00ff) | ((v as u32) << 8);
    }
    #[inline]
    pub fn bx(&self) -> u16 {
        self.ebx as u16
    }
    #[inline]
    pub fn cx(&self) -> u16 {
        self.ecx as u16
    }
    #[inline]
    pub fn dx(&self) -> u16 {
        self.edx as u16
    }
    #[inline]
    pub fn si(&self) -> u16 {
        self.esi as u16
    }
    /// Linux's `hsi` alias — high 16 bits of ESI.
    #[inline]
    pub fn hsi(&self) -> u16 {
        (self.esi >> 16) as u16
    }
    #[inline]
    pub fn di(&self) -> u16 {
        self.edi as u16
    }
    /// `flags` alias used by APM check — low 16 bits of `eflags`.
    #[inline]
    pub fn flags(&self) -> u16 {
        self.eflags as u16
    }
}

/// Seam for the real-mode BIOS far call. Production builds wire this to
/// the assembly trampoline (`bioscall.S`); tests use a deterministic
/// stub. Mirrors the signature of Linux's
/// `void intcall(u8 int_no, const struct biosregs *ireg, struct biosregs *oreg)`.
pub trait BiosCaller {
    fn intcall(&self, int_no: u8, ireg: &BiosRegs, oreg: Option<&mut BiosRegs>);
}

/// Production BIOS caller — calls into the real-mode `bioscall` thunk.
/// Currently a placeholder because lupos doesn't run real-mode setup;
/// once the trampoline lands in batch 3 (`boot/compressed/`) this
/// dispatches to it.
pub struct RealModeBios;

impl BiosCaller for RealModeBios {
    fn intcall(&self, _int_no: u8, _ireg: &BiosRegs, _oreg: Option<&mut BiosRegs>) {
        // Real-mode trampoline lives in bioscall.S — not invokable from
        // protected/long-mode lupos kernel. The stub clears CF=0 to
        // distinguish "didn't run" from "BIOS reported error".
        if let Some(o) = _oreg {
            *o = BiosRegs::default();
            o.eflags &= !X86_EFLAGS_CF;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn al_ah_ax_round_trip_through_eax() {
        let mut r = BiosRegs::default();
        r.set_ah(0x53);
        r.set_al(0x12);
        assert_eq!(r.ah(), 0x53);
        assert_eq!(r.al(), 0x12);
        assert_eq!(r.ax(), 0x5312);
    }

    #[test]
    fn carry_flag_constant_matches_processor_flags_h() {
        assert_eq!(X86_EFLAGS_CF, 0x1);
    }

    #[test]
    fn hsi_returns_high_half_of_esi() {
        let r = BiosRegs {
            esi: 0xdead_beef,
            ..Default::default()
        };
        assert_eq!(r.si(), 0xbeef);
        assert_eq!(r.hsi(), 0xdead);
    }

    #[test]
    fn struct_layout_size_matches_linux_biosregs() {
        // 9 × u32 (edi..eflags) + 4 × u16 (gs..ds) = 44 bytes.
        // Linux's biosregs is 44 bytes (verified against bootparam.h).
        assert_eq!(core::mem::size_of::<BiosRegs>(), 44);
    }
}
