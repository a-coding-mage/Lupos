//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/apm.c
//! test-origin: linux:vendor/linux/arch/x86/boot/apm.c
//! APM BIOS query sequence.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/apm.c
//!
//! Real-mode setup probes for APM (Advanced Power Management) via
//! INT 15h AX=53xx. Linux runs the install-check (AH=53h, AL=00h),
//! verifies the "PM" signature in BX = 0x504D, and that 32-bit support
//! is advertised (CX bit 1). If accepted, it disconnects and then
//! re-connects in 32-bit mode, populating the boot-params APM block.
//!
//! Lupos has no real-mode runtime, so the seam is a `BiosCaller` trait;
//! the algorithmic flow and signature checks are byte-faithful.

use super::biosregs::{BiosCaller, BiosRegs, X86_EFLAGS_CF};
use super::regs::initregs;

/// Linux's APM info block subset populated by `query_apm_bios()`.
/// Layout matches `boot_params.apm_bios_info` in
/// `vendor/linux/arch/x86/include/uapi/asm/bootparam.h::apm_bios_info`.
#[repr(C)]
#[derive(Copy, Clone, Default, Debug, Eq, PartialEq)]
pub struct ApmBiosInfo {
    pub version: u16,
    pub cseg: u16,
    pub offset: u32,
    pub cseg_16: u16,
    pub dseg: u16,
    pub flags: u16,
    pub cseg_len: u16,
    pub cseg_16_len: u16,
    pub dseg_len: u16,
}

/// `query_apm_bios()` — return the populated info block on success.
/// Mirrors apm.c line-by-line.
pub fn query_apm_bios<B: BiosCaller>(bios: &B) -> Result<ApmBiosInfo, ()> {
    let mut ireg = BiosRegs::default();
    let mut oreg = BiosRegs::default();

    // APM BIOS installation check (AH=0x53, AL=0x00).
    initregs(&mut ireg);
    ireg.set_ah(0x53);
    bios.intcall(0x15, &ireg, Some(&mut oreg));

    if oreg.flags() as u32 & X86_EFLAGS_CF != 0 {
        return Err(());
    }
    if oreg.bx() != 0x504d {
        return Err(());
    }
    if oreg.cx() & 0x02 == 0 {
        return Err(());
    }

    // Disconnect first (AL=0x04) in case of stale connect.
    ireg.set_al(0x04);
    bios.intcall(0x15, &ireg, None);

    // 32-bit connect (AL=0x03).
    ireg.set_al(0x03);
    bios.intcall(0x15, &ireg, Some(&mut oreg));

    let mut info = ApmBiosInfo {
        cseg: oreg.ax(),
        offset: oreg.ebx,
        cseg_16: oreg.cx(),
        dseg: oreg.dx(),
        cseg_len: oreg.si(),
        cseg_16_len: oreg.hsi(),
        dseg_len: oreg.di(),
        ..Default::default()
    };

    if oreg.flags() as u32 & X86_EFLAGS_CF != 0 {
        return Err(());
    }

    // Redo installation check as the 32-bit connect.
    ireg.set_al(0x00);
    bios.intcall(0x15, &ireg, Some(&mut oreg));

    if oreg.eflags & X86_EFLAGS_CF != 0 || oreg.bx() != 0x504d {
        // Failure: disconnect and bail.
        ireg.set_al(0x04);
        bios.intcall(0x15, &ireg, None);
        return Err(());
    }

    info.version = oreg.ax();
    info.flags = oreg.cx();
    Ok(info)
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::RefCell;

    struct Stub {
        // FIFO of (cf, bx, cx, ax, ebx, dx, si, hsi, di, eflags) per call.
        replies: RefCell<alloc::vec::Vec<BiosRegs>>,
        last_calls: RefCell<alloc::vec::Vec<(u8, u8)>>, // (int_no, al)
    }
    impl BiosCaller for Stub {
        fn intcall(&self, int_no: u8, ireg: &BiosRegs, oreg: Option<&mut BiosRegs>) {
            self.last_calls.borrow_mut().push((int_no, ireg.al()));
            if let Some(out) = oreg {
                if let Some(reply) = self.replies.borrow_mut().pop() {
                    *out = reply;
                }
            }
        }
    }
    fn stub_with(replies: alloc::vec::Vec<BiosRegs>) -> Stub {
        let mut r = replies;
        r.reverse();
        Stub {
            replies: RefCell::new(r),
            last_calls: RefCell::new(alloc::vec::Vec::new()),
        }
    }

    extern crate alloc;

    fn ok_install_reply() -> BiosRegs {
        let mut r = BiosRegs::default();
        // CF clear, BX="PM" (0x504d), CX has 32-bit bit 1 set.
        r.eflags = 0;
        r.ebx = 0x504d;
        r.ecx = 0x02;
        r
    }
    fn ok_connect_reply() -> BiosRegs {
        // 32-bit connect: AX=cseg, EBX=offset, CX=cseg_16, DX=dseg,
        // SI=cseg_len, hsi=cseg_16_len, DI=dseg_len. CF clear.
        let mut r = BiosRegs::default();
        r.eflags = 0;
        r.eax = 0x1234;
        r.ebx = 0xdead_beef;
        r.ecx = 0x4321;
        r.edx = 0x9999;
        r.esi = (0x7777u32 << 16) | 0x5555u32;
        r.edi = 0x3333;
        r
    }
    fn ok_recheck_reply() -> BiosRegs {
        let mut r = BiosRegs::default();
        r.eflags = 0;
        r.ebx = 0x504d;
        r.eax = 0x0102; // APM 1.2
        r.ecx = 0x0003; // flags
        r
    }

    #[test]
    fn query_apm_succeeds_when_signatures_match() {
        let stub = stub_with(alloc::vec![
            ok_install_reply(),
            ok_connect_reply(),
            ok_recheck_reply(),
        ]);
        let info = query_apm_bios(&stub).expect("APM query should succeed");
        assert_eq!(info.cseg, 0x1234);
        assert_eq!(info.offset, 0xdead_beef);
        assert_eq!(info.cseg_16, 0x4321);
        assert_eq!(info.dseg, 0x9999);
        assert_eq!(info.cseg_len, 0x5555);
        assert_eq!(info.cseg_16_len, 0x7777);
        assert_eq!(info.dseg_len, 0x3333);
        assert_eq!(info.version, 0x0102);
        assert_eq!(info.flags, 0x0003);
    }

    #[test]
    fn query_apm_fails_when_signature_is_wrong() {
        let mut bad = ok_install_reply();
        bad.ebx = 0x4D50; // "MP" — wrong endianness
        assert_eq!(query_apm_bios(&stub_with(alloc::vec![bad])), Err(()));
    }

    #[test]
    fn query_apm_fails_when_32_bit_not_advertised() {
        let mut no32 = ok_install_reply();
        no32.ecx = 0x00;
        assert_eq!(query_apm_bios(&stub_with(alloc::vec![no32])), Err(()));
    }

    #[test]
    fn apm_bios_info_layout_size_matches_linux() {
        // Linux apm_bios_info: 2+2+4+2+2+2+2+2+2 = 20 bytes.
        assert_eq!(core::mem::size_of::<ApmBiosInfo>(), 20);
    }
}
