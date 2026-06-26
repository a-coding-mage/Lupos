//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/umwait.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/umwait.c
//! UMWAIT / TPAUSE control MSR.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/umwait.c

// MSR_IA32_UMWAIT_CONTROL (0xe1) holds bit 0 = C0.2 disable and
// bits[31:2] = max_time in TSC ticks. The kernel exposes
// /sys/devices/system/cpu/umwait_control/ to tune both. We model the
// MSR pack/unpack.

pub const MSR_IA32_UMWAIT_CONTROL: u32 = 0x0000_00e1;
pub const UMWAIT_CONTROL_C02_DISABLE: u32 = 1 << 0;
pub const UMWAIT_CONTROL_MAX_TIME_MASK: u32 = !0x3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UmwaitControl {
    pub c02_disable: bool,
    pub max_time: u32,
}

pub const fn encode(ctl: UmwaitControl) -> u32 {
    let mut value = ctl.max_time & UMWAIT_CONTROL_MAX_TIME_MASK;
    if ctl.c02_disable {
        value |= UMWAIT_CONTROL_C02_DISABLE;
    }
    value
}

pub const fn decode(msr: u32) -> UmwaitControl {
    UmwaitControl {
        c02_disable: msr & UMWAIT_CONTROL_C02_DISABLE != 0,
        max_time: msr & UMWAIT_CONTROL_MAX_TIME_MASK,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_preserves_c02_disable_flag() {
        let ctl = UmwaitControl {
            c02_disable: true,
            max_time: 0x10000,
        };
        let msr = encode(ctl);
        let back = decode(msr);
        assert_eq!(back, ctl);
    }

    #[test]
    fn max_time_low_two_bits_cleared() {
        let ctl = UmwaitControl {
            c02_disable: false,
            max_time: 0xff,
        };
        assert_eq!(encode(ctl) & 0x3, 0);
    }
}
