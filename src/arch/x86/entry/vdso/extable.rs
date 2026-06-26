//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/entry/vdso/extable.c
//! test-origin: linux:vendor/linux/arch/x86/entry/vdso/extable.c
//! vDSO exception-table fixups.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/entry/vdso/extable.c

pub const X86_TRAP_DB: i32 = 1;
pub const X86_TRAP_BP: i32 = 3;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VdsoExceptionTableEntry {
    pub insn: i32,
    pub fixup: i32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VdsoExceptionRegs {
    pub ip: u64,
    pub di: u64,
    pub si: u64,
    pub dx: u64,
}

pub fn fixup_vdso_exception(
    regs: &mut VdsoExceptionRegs,
    trapnr: i32,
    error_code: u64,
    fault_addr: u64,
    vdso_base: Option<u64>,
    extable_base: u64,
    entries: &[VdsoExceptionTableEntry],
) -> bool {
    if trapnr == X86_TRAP_DB || trapnr == X86_TRAP_BP {
        return false;
    }
    let Some(vdso) = vdso_base else {
        return false;
    };
    let base = vdso + extable_base;
    for entry in entries {
        if regs.ip == base.wrapping_add(entry.insn as u64) {
            regs.ip = base.wrapping_add(entry.fixup as u64);
            regs.di = trapnr as u64;
            regs.si = error_code;
            regs.dx = fault_addr;
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixup_updates_ip_and_argument_registers() {
        let mut regs = VdsoExceptionRegs {
            ip: 0x1100,
            ..Default::default()
        };
        let entries = [VdsoExceptionTableEntry {
            insn: 0x100,
            fixup: 0x180,
        }];
        assert!(fixup_vdso_exception(
            &mut regs,
            14,
            0x22,
            0xdead,
            Some(0x1000),
            0,
            &entries
        ));
        assert_eq!(regs.ip, 0x1180);
        assert_eq!(regs.di, 14);
        assert_eq!(regs.si, 0x22);
        assert_eq!(regs.dx, 0xdead);
    }

    #[test]
    fn db_and_bp_are_never_fixed_up() {
        let mut regs = VdsoExceptionRegs {
            ip: 0x1100,
            ..Default::default()
        };
        assert!(!fixup_vdso_exception(
            &mut regs,
            X86_TRAP_BP,
            0,
            0,
            Some(0x1000),
            0,
            &[VdsoExceptionTableEntry {
                insn: 0x100,
                fixup: 0x180
            }]
        ));
    }
}
