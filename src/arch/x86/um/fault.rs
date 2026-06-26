//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/um/fault.c
//! test-origin: linux:vendor/linux/arch/x86/um/fault.c
//! UML exception-table fixup handling.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExceptionTableEntry {
    pub insn: usize,
    pub fixup: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UmlPtRegs {
    pub ip: usize,
}

pub fn search_exception_tables(
    address: usize,
    table: &[ExceptionTableEntry],
) -> Option<ExceptionTableEntry> {
    table.iter().copied().find(|entry| entry.insn == address)
}

pub fn arch_fixup(address: usize, regs: &mut UmlPtRegs, table: &[ExceptionTableEntry]) -> i32 {
    if let Some(fixup) = search_exception_tables(address, table) {
        regs.ip = fixup.fixup;
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arch_fixup_sets_ip_when_exception_entry_exists() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/um/fault.c"
        ));
        assert!(source.contains("UPT_IP(regs) = fixup->fixup;"));
        assert!(source.contains("return 1;"));
        assert!(source.contains("return 0;"));

        let table = [
            ExceptionTableEntry {
                insn: 0x1000,
                fixup: 0x2000,
            },
            ExceptionTableEntry {
                insn: 0x3000,
                fixup: 0x4000,
            },
        ];
        let mut regs = UmlPtRegs { ip: 0xaaaa };
        assert_eq!(arch_fixup(0x3000, &mut regs, &table), 1);
        assert_eq!(regs.ip, 0x4000);

        assert_eq!(arch_fixup(0xbeef, &mut regs, &table), 0);
        assert_eq!(regs.ip, 0x4000);
    }
}
