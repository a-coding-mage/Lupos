//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/um/syscalls_32.c
//! test-origin: linux:vendor/linux/arch/x86/um/syscalls_32.c
//! UML i386 syscall shims.

use crate::include::uapi::errno::EINVAL;

pub const fn arch_prctl(_option: i32, _arg2: usize) -> i32 {
    -EINVAL
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arch_prctl_is_not_supported_on_um_i386() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/um/syscalls_32.c"
        ));
        assert!(source.contains("return -EINVAL;"));

        assert_eq!(arch_prctl(0, 0), -EINVAL);
        assert_eq!(arch_prctl(0x1002, 0xdead_beef), -EINVAL);
    }
}
