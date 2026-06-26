//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/lib/kaslr.c
//! test-origin: linux:vendor/linux/arch/x86/lib/kaslr.c
//! x86 KASLR entropy mixing helpers.

pub const MIX_CONST_64: u64 = 0x5d6008cbf3848dd3;
pub const MIX_CONST_32: u32 = 0x3f39e593;
pub const I8254_PORT_CONTROL: u16 = 0x43;
pub const I8254_PORT_COUNTER0: u16 = 0x40;
pub const I8254_CMD_READBACK: u8 = 0xc0;
pub const I8254_SELECT_COUNTER0: u8 = 0x02;
pub const I8254_STATUS_NOTREADY: u8 = 0x40;

pub const fn i8254_readback_command() -> u8 {
    I8254_CMD_READBACK | I8254_SELECT_COUNTER0
}

pub const fn i8254_ready(status: u8) -> bool {
    status & I8254_STATUS_NOTREADY == 0
}

pub const fn i8254_timer_value(lo: u8, hi: u8) -> u16 {
    ((hi as u16) << 8) | lo as u16
}

pub const fn kaslr_should_use_i8254(rdrand_ok: bool, has_tsc: bool) -> bool {
    !rdrand_ok && !has_tsc
}

pub fn kaslr_mix_random(random: u64) -> u64 {
    let product = (random as u128).wrapping_mul(MIX_CONST_64 as u128);
    (product as u64).wrapping_add((product >> 64) as u64)
}

pub fn kaslr_fold_entropy(
    boot_seed: u64,
    rdrand: Option<u64>,
    tsc: Option<u64>,
    i8254: u16,
) -> u64 {
    let mut random = boot_seed;
    let mut use_i8254 = true;
    if let Some(raw) = rdrand {
        random ^= raw;
        use_i8254 = false;
    }
    if let Some(raw) = tsc {
        random ^= raw;
        use_i8254 = false;
    }
    if use_i8254 {
        random ^= u64::from(i8254);
    }
    kaslr_mix_random(random)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kaslr_entropy_mixer_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/lib/kaslr.c"
        ));
        assert!(source.contains("#define I8254_PORT_CONTROL\t0x43"));
        assert!(source.contains("#define I8254_PORT_COUNTER0\t0x40"));
        assert!(source.contains("#define I8254_CMD_READBACK\t0xC0"));
        assert!(source.contains("#define I8254_SELECT_COUNTER0\t0x02"));
        assert!(source.contains("#define I8254_STATUS_NOTREADY\t0x40"));
        assert!(source.contains("const unsigned long mix_const = 0x5d6008cbf3848dd3UL;"));
        assert!(source.contains("const unsigned long mix_const = 0x3f39e593UL;"));
        assert!(source.contains("bool use_i8254 = true;"));
        assert!(source.contains("random ^= raw;"));
        assert!(source.contains("use_i8254 = false;"));
        assert!(source.contains("random ^= i8254();"));
        assert!(source.contains("/* Circular multiply for better bit diffusion */"));
        assert!(source.contains("random += raw;"));

        assert_eq!(i8254_readback_command(), 0xc2);
        assert!(i8254_ready(0));
        assert!(!i8254_ready(I8254_STATUS_NOTREADY));
        assert_eq!(i8254_timer_value(0x34, 0x12), 0x1234);
        assert!(kaslr_should_use_i8254(false, false));
        assert!(!kaslr_should_use_i8254(true, false));
        assert_eq!(kaslr_mix_random(1), MIX_CONST_64);

        let with_rdrand = kaslr_fold_entropy(0, Some(0x1234), None, 0xdead);
        let with_rdrand_other_pit = kaslr_fold_entropy(0, Some(0x1234), None, 0xbeef);
        assert_eq!(with_rdrand, with_rdrand_other_pit);
        assert_ne!(
            kaslr_fold_entropy(0, None, None, 0xdead),
            kaslr_fold_entropy(0, None, None, 0xbeef)
        );
    }
}
