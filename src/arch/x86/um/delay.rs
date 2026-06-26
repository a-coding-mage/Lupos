//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/um/delay.c
//! test-origin: linux:vendor/linux/arch/x86/um/delay.c
//! UML delay loop scaling.

pub const UDELAY_MULTIPLIER: u64 = 0x0000_10c7;
pub const NDELAY_MULTIPLIER: u64 = 0x0000_0005;

pub const fn const_udelay_loops(xloops: u64, loops_per_jiffy: u64, hz: u64) -> u64 {
    let scaled = xloops.saturating_mul(4);
    let product = scaled.saturating_mul(loops_per_jiffy.saturating_mul(hz / 4));
    (product >> 32).saturating_add(1)
}

pub const fn udelay_xloops(usecs: u64) -> u64 {
    usecs.saturating_mul(UDELAY_MULTIPLIER)
}

pub const fn ndelay_xloops(nsecs: u64) -> u64 {
    nsecs.saturating_mul(NDELAY_MULTIPLIER)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uml_delay_scaling_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/um/delay.c"
        ));
        assert!(source.contains("void __delay(unsigned long loops)"));
        assert!(source.contains("xloops *= 4;"));
        assert!(source.contains("loops_per_jiffy * (HZ/4)"));
        assert!(source.contains("__delay(++xloops);"));
        assert!(source.contains("__const_udelay(usecs * 0x000010c7);"));
        assert!(source.contains("__const_udelay(nsecs * 0x00005);"));
        assert!(source.contains("EXPORT_SYMBOL(__udelay);"));
        assert!(source.contains("EXPORT_SYMBOL(__ndelay);"));

        assert_eq!(udelay_xloops(2), 2 * 0x10c7);
        assert_eq!(ndelay_xloops(3), 15);
        assert_eq!(const_udelay_loops(1 << 30, 4, 4), 5);
    }
}
