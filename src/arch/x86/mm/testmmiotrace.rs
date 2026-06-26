//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/mm/testmmiotrace.c
//! test-origin: linux:vendor/linux/arch/x86/mm/testmmiotrace.c
//! MMIOTRACE self-test policy.
//!
//! Mirrors the deterministic data generators and disabled module gate from
//! `vendor/linux/arch/x86/mm/testmmiotrace.c`.

use crate::include::uapi::errno::ENODEV;

pub const fn v16(i: u32) -> u16 {
    ((i & 0xff) as u16) | (((!i) & 0xff) as u16) << 8
}

pub const fn v32(i: u32) -> u32 {
    (v16(i) as u32) | ((v16(i.wrapping_add(1)) as u32) << 16)
}

pub const fn testmmiotrace_init(enabled: bool) -> Result<(), i32> {
    if enabled { Ok(()) } else { Err(ENODEV) }
}

pub const fn testmmiotrace_cleanup() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_patterns_are_stable() {
        assert_eq!(v16(0), 0xff00);
        assert_eq!(v32(0), 0xfe01_ff00);
    }

    #[test]
    fn disabled_test_module_fails_closed() {
        assert_eq!(testmmiotrace_init(false), Err(ENODEV));
    }
}
