//! linux-parity: complete
//! linux-source: vendor/linux/lib/math/reciprocal_div.c
//! test-origin: linux:vendor/linux/lib/math/reciprocal_div.c
//! Reciprocal divisor precomputation helpers.

use crate::kernel::module::{export_symbol, find_symbol};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReciprocalValue {
    pub m: u32,
    pub sh1: u8,
    pub sh2: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReciprocalValueAdv {
    pub m: u32,
    pub sh: u8,
    pub exp: u8,
    pub is_wide_m: bool,
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("reciprocal_value", reciprocal_value_raw as usize, false);
    export_symbol_once(
        "reciprocal_value_adv",
        reciprocal_value_adv_raw as usize,
        false,
    );
}

const fn fls_u32(value: u32) -> u32 {
    if value == 0 {
        0
    } else {
        u32::BITS - value.leading_zeros()
    }
}

pub const fn reciprocal_value(d: u32) -> ReciprocalValue {
    if d == 0 {
        return ReciprocalValue {
            m: 0,
            sh1: 0,
            sh2: 0,
        };
    }

    let l = fls_u32(d - 1);
    let mut m = ((1u128 << 32) * ((1u128 << l) - d as u128)) / d as u128;
    m += 1;
    ReciprocalValue {
        m: m as u32,
        sh1: if l < 1 { l as u8 } else { 1 },
        sh2: l.saturating_sub(1) as u8,
    }
}

pub const fn reciprocal_divide(a: u32, reciprocal: ReciprocalValue) -> u32 {
    let t = (((a as u64) * (reciprocal.m as u64)) >> 32) as u32;
    (t + ((a - t) >> reciprocal.sh1)) >> reciprocal.sh2
}

pub const fn reciprocal_value_adv(d: u32, prec: u8) -> ReciprocalValueAdv {
    if d == 0 {
        return ReciprocalValueAdv {
            m: 0,
            sh: 0,
            exp: 0,
            is_wide_m: false,
        };
    }

    let l = fls_u32(d - 1);
    if l >= 32 {
        return ReciprocalValueAdv {
            m: 0,
            sh: l as u8,
            exp: l as u8,
            is_wide_m: true,
        };
    }

    let mut post_shift = l;
    let mut mlow = (1u128 << (32 + l)) / d as u128;
    let extra_shift = (32 + l).saturating_sub(prec as u32);
    let mut mhigh = ((1u128 << (32 + l)) + (1u128 << extra_shift)) / d as u128;

    while post_shift > 0 {
        let lo = mlow >> 1;
        let hi = mhigh >> 1;
        if lo >= hi {
            break;
        }
        mlow = lo;
        mhigh = hi;
        post_shift -= 1;
    }

    ReciprocalValueAdv {
        m: mhigh as u32,
        sh: post_shift as u8,
        exp: l as u8,
        is_wide_m: mhigh > u32::MAX as u128,
    }
}

pub extern "C" fn reciprocal_value_raw(d: u32) -> ReciprocalValue {
    reciprocal_value(d)
}

pub extern "C" fn reciprocal_value_adv_raw(d: u32, prec: u8) -> ReciprocalValueAdv {
    reciprocal_value_adv(d, prec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reciprocal_value_matches_linux_basic_formula() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/math/reciprocal_div.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/reciprocal_div.h"
        ));
        assert!(source.contains("l = fls(d - 1);"));
        assert!(source.contains("m = ((1ULL << 32) * ((1ULL << l) - d));"));
        assert!(source.contains("R.sh1 = min(l, 1);"));
        assert!(source.contains("R.sh2 = max(l - 1, 0);"));
        assert!(header.contains("(t + ((a - t) >> R.sh1)) >> R.sh2"));

        for divisor in [1u32, 3, 7, 10, 1024, 65535] {
            let reciprocal = reciprocal_value(divisor);
            for dividend in [0u32, 1, divisor - 1, divisor, u16::MAX as u32, u32::MAX] {
                assert_eq!(reciprocal_divide(dividend, reciprocal), dividend / divisor);
            }
        }
    }

    #[test]
    fn reciprocal_value_adv_follows_linux_shift_reduction() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/math/reciprocal_div.c"
        ));
        assert!(source.contains("mlow = 1ULL << (32 + l);"));
        assert!(source.contains("mhigh = (1ULL << (32 + l)) + (1ULL << (32 + l - prec));"));
        assert!(source.contains("for (; post_shift > 0; post_shift--)"));
        assert!(source.contains("R.is_wide_m = mhigh > U32_MAX;"));

        assert_eq!(
            reciprocal_value_adv(3, 32),
            ReciprocalValueAdv {
                m: 0xaaaa_aaab,
                sh: 1,
                exp: 2,
                is_wide_m: false,
            }
        );
        assert_eq!(reciprocal_value_adv(10, 32).exp, 4);
    }
}
