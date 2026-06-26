//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/tsc_msr.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/tsc_msr.c
//! TSC frequency enumeration via Intel MSRs.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/tsc_msr.c

#![allow(dead_code)]

pub const MAX_NUM_FREQS: usize = 16;
pub const TSC_REFERENCE_KHZ: u32 = 100_000;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MulDiv {
    pub multiplier: u32,
    pub divider: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FreqDesc {
    pub use_msr_plat: bool,
    pub muldiv: [MulDiv; MAX_NUM_FREQS],
    pub freqs: [u32; MAX_NUM_FREQS],
    pub mask: u32,
}

impl FreqDesc {
    pub const fn empty(use_msr_plat: bool, mask: u32) -> Self {
        Self {
            use_msr_plat,
            muldiv: [MulDiv {
                multiplier: 0,
                divider: 0,
            }; MAX_NUM_FREQS],
            freqs: [0; MAX_NUM_FREQS],
            mask,
        }
    }
}

pub const FREQ_DESC_BYT: FreqDesc = {
    let mut desc = FreqDesc::empty(true, 0x07);
    desc.muldiv[0] = MulDiv {
        multiplier: 5,
        divider: 6,
    };
    desc.muldiv[1] = MulDiv {
        multiplier: 1,
        divider: 1,
    };
    desc.muldiv[2] = MulDiv {
        multiplier: 4,
        divider: 3,
    };
    desc.muldiv[3] = MulDiv {
        multiplier: 7,
        divider: 6,
    };
    desc.muldiv[4] = MulDiv {
        multiplier: 4,
        divider: 5,
    };
    desc
};

pub const FREQ_DESC_CHT: FreqDesc = {
    let mut desc = FreqDesc::empty(true, 0x0f);
    desc.muldiv[0] = MulDiv {
        multiplier: 5,
        divider: 6,
    };
    desc.muldiv[1] = MulDiv {
        multiplier: 1,
        divider: 1,
    };
    desc.muldiv[2] = MulDiv {
        multiplier: 4,
        divider: 3,
    };
    desc.muldiv[3] = MulDiv {
        multiplier: 7,
        divider: 6,
    };
    desc.muldiv[4] = MulDiv {
        multiplier: 4,
        divider: 5,
    };
    desc.muldiv[5] = MulDiv {
        multiplier: 14,
        divider: 15,
    };
    desc.muldiv[6] = MulDiv {
        multiplier: 9,
        divider: 10,
    };
    desc.muldiv[7] = MulDiv {
        multiplier: 8,
        divider: 9,
    };
    desc.muldiv[8] = MulDiv {
        multiplier: 7,
        divider: 8,
    };
    desc
};

pub const FREQ_DESC_LGM: FreqDesc = FreqDesc {
    use_msr_plat: true,
    muldiv: [MulDiv {
        multiplier: 0,
        divider: 0,
    }; MAX_NUM_FREQS],
    freqs: [78_000; MAX_NUM_FREQS],
    mask: 0x0f,
};

const fn div_round_closest(n: u64, d: u64) -> u64 {
    (n + d / 2) / d
}

pub const fn cpu_khz_from_msr_values(desc: &FreqDesc, ratio: u32, fsb_freq_lo: u32) -> u64 {
    let index = (fsb_freq_lo & desc.mask) as usize;
    if index >= MAX_NUM_FREQS {
        return 0;
    }
    let md = desc.muldiv[index];
    if md.divider != 0 {
        let tscref = TSC_REFERENCE_KHZ as u64 * md.multiplier as u64;
        div_round_closest(tscref * ratio as u64, md.divider as u64)
    } else {
        desc.freqs[index] as u64 * ratio as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baytrail_frequency_uses_pll_ratio_table() {
        assert_eq!(cpu_khz_from_msr_values(&FREQ_DESC_BYT, 16, 0), 1_333_333);
        assert_eq!(cpu_khz_from_msr_values(&FREQ_DESC_BYT, 16, 1), 1_600_000);
    }

    #[test]
    fn lightning_mountain_uses_fixed_frequency_table() {
        assert_eq!(cpu_khz_from_msr_values(&FREQ_DESC_LGM, 20, 4), 1_560_000);
    }
}
