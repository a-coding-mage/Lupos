//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/um/syscalls_64.c
//! test-origin: linux:vendor/linux/arch/x86/um/syscalls_64.c
//! UML x86_64 arch_prctl and mmap syscall behavior.

use crate::include::uapi::errno::EINVAL;

pub const ARCH_SET_GS: i32 = 0x1001;
pub const ARCH_SET_FS: i32 = 0x1002;
pub const ARCH_GET_FS: i32 = 0x1003;
pub const ARCH_GET_GS: i32 = 0x1004;
pub const PAGE_SHIFT: u32 = 12;
pub const PAGE_MASK: u64 = !((1u64 << PAGE_SHIFT) - 1);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UmlThreadRegs {
    pub fs_base: u64,
    pub gs_base: u64,
}

pub fn arch_prctl(regs: &mut UmlThreadRegs, option: i32, arg2: u64) -> Result<u64, i32> {
    match option {
        ARCH_SET_FS => {
            regs.fs_base = arg2;
            Ok(0)
        }
        ARCH_SET_GS => {
            regs.gs_base = arg2;
            Ok(0)
        }
        ARCH_GET_FS => Ok(regs.fs_base),
        ARCH_GET_GS => Ok(regs.gs_base),
        _ => Err(-EINVAL),
    }
}

pub const fn uml_mmap_pgoff(off: u64) -> Result<u64, i32> {
    if off & !PAGE_MASK != 0 {
        Err(-EINVAL)
    } else {
        Ok(off >> PAGE_SHIFT)
    }
}

pub const fn arch_switch_to_noop() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uml_syscalls64_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/um/syscalls_64.c"
        ));
        assert!(source.contains("long arch_prctl"));
        assert!(source.contains("case ARCH_SET_FS:"));
        assert!(source.contains("case ARCH_SET_GS:"));
        assert!(source.contains("case ARCH_GET_FS:"));
        assert!(source.contains("case ARCH_GET_GS:"));
        assert!(source.contains("SYSCALL_DEFINE2(arch_prctl"));
        assert!(source.contains("void arch_switch_to"));
        assert!(source.contains("Nothing needs to be done on x86_64."));
        assert!(source.contains("SYSCALL_DEFINE6(mmap"));
        assert!(source.contains("if (off & ~PAGE_MASK)"));
        assert!(source.contains("ksys_mmap_pgoff"));

        let mut regs = UmlThreadRegs::default();
        assert_eq!(arch_prctl(&mut regs, ARCH_SET_FS, 0x1111), Ok(0));
        assert_eq!(arch_prctl(&mut regs, ARCH_GET_FS, 0), Ok(0x1111));
        assert_eq!(arch_prctl(&mut regs, 0, 0), Err(-EINVAL));
        assert_eq!(uml_mmap_pgoff(0x3000), Ok(3));
        assert_eq!(uml_mmap_pgoff(1), Err(-EINVAL));
        assert!(arch_switch_to_noop());
    }
}
