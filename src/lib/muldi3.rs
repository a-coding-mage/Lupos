//! linux-parity: complete
//! linux-source: vendor/linux/lib/muldi3.c
//! test-origin: linux:vendor/linux/lib/muldi3.c
//! Signed 64-bit libgcc multiplication helper.

use crate::kernel::module::{export_symbol, find_symbol};

pub const MULDI3_EXPORT_SYMBOL: &str = "__muldi3";
pub const W_TYPE_SIZE: u32 = 32;
pub const LL_B: u64 = 1u64 << (W_TYPE_SIZE / 2);
pub const LL_LOW_MASK: u64 = LL_B - 1;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(MULDI3_EXPORT_SYMBOL, __muldi3 as usize, false);
}

pub const fn ll_lowpart(t: u32) -> u64 {
    (t as u64) & LL_LOW_MASK
}

pub const fn ll_highpart(t: u32) -> u64 {
    (t as u64) >> (W_TYPE_SIZE / 2)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct U32Product {
    pub high: u32,
    pub low: u32,
}

pub fn umul_ppmm(u: u32, v: u32) -> U32Product {
    let ul = ll_lowpart(u);
    let uh = ll_highpart(u);
    let vl = ll_lowpart(v);
    let vh = ll_highpart(v);

    let x0 = ul * vl;
    let mut x1 = ul * vh;
    let x2 = uh * vl;
    let mut x3 = uh * vh;

    x1 += x0 >> (W_TYPE_SIZE / 2);
    x1 += x2;
    if x1 < x2 {
        x3 += LL_B;
    }

    U32Product {
        high: (x3 + (x1 >> (W_TYPE_SIZE / 2))) as u32,
        low: (((x1 & LL_LOW_MASK) * LL_B) + (x0 & LL_LOW_MASK)) as u32,
    }
}

pub fn umulsidi3(u: u32, v: u32) -> u64 {
    let product = umul_ppmm(u, v);
    ((product.high as u64) << W_TYPE_SIZE) | product.low as u64
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DwParts {
    pub low: u32,
    pub high: u32,
}

pub fn dw_parts(value: i64) -> DwParts {
    let raw = value as u64;
    DwParts {
        low: raw as u32,
        high: (raw >> W_TYPE_SIZE) as u32,
    }
}

pub fn dw_from_parts(parts: DwParts) -> i64 {
    (((parts.high as u64) << W_TYPE_SIZE) | parts.low as u64) as i64
}

pub fn muldi3_by_words(u: i64, v: i64) -> i64 {
    let uu = dw_parts(u);
    let vv = dw_parts(v);
    let mut w = dw_parts(umulsidi3(uu.low, vv.low) as i64);
    let cross = (uu.low as u64)
        .wrapping_mul(vv.high as u64)
        .wrapping_add((uu.high as u64).wrapping_mul(vv.low as u64));
    w.high = w.high.wrapping_add(cross as u32);
    dw_from_parts(w)
}

pub extern "C" fn __muldi3(u: i64, v: i64) -> i64 {
    muldi3_by_words(u, v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn muldi3_matches_linux_source_and_wrapping_product() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/muldi3.c"
        ));
        assert!(source.contains("#define W_TYPE_SIZE 32"));
        assert!(source.contains("#define __ll_B ((unsigned long) 1 << (W_TYPE_SIZE / 2))"));
        assert!(source.contains("#define __ll_lowpart(t)"));
        assert!(source.contains("#define __ll_highpart(t)"));
        assert!(source.contains("unsigned short __ul, __vl, __uh, __vh;"));
        assert!(source.contains("__x1 += __ll_highpart(__x0);"));
        assert!(source.contains("if (__x1 < __x2)"));
        assert!(source.contains("umul_ppmm(__w.s.high, __w.s.low, u, v);"));
        assert!(source.contains("const DWunion uu = {.ll = u};"));
        assert!(source.contains("const DWunion vv = {.ll = v};"));
        assert!(source.contains("w.s.high += ((unsigned long) uu.s.low"));
        assert!(source.contains("EXPORT_SYMBOL(__muldi3);"));

        assert_eq!(MULDI3_EXPORT_SYMBOL, "__muldi3");
        assert_eq!(W_TYPE_SIZE, 32);
        assert_eq!(LL_B, 65_536);
        assert_eq!(LL_LOW_MASK, 0xffff);
        assert_eq!(ll_lowpart(0xabcd_1234), 0x1234);
        assert_eq!(ll_highpart(0xabcd_1234), 0xabcd);
        assert_eq!(
            umul_ppmm(0xffff_ffff, 0xffff_ffff),
            U32Product {
                high: 0xffff_fffe,
                low: 1
            }
        );
        assert_eq!(
            umulsidi3(0x1234_5678, 0x9abc_def0),
            (0x1234_5678u64).wrapping_mul(0x9abc_def0)
        );
        assert_eq!(
            dw_parts(0x1122_3344_5566_7788i64),
            DwParts {
                high: 0x1122_3344,
                low: 0x5566_7788
            }
        );
        assert_eq!(
            dw_from_parts(DwParts {
                high: 0x89ab_cdef,
                low: 0x0123_4567
            }) as u64,
            0x89ab_cdef_0123_4567
        );

        assert_eq!(__muldi3(7, 9), 63);
        assert_eq!(__muldi3(-7, 9), -63);
        assert_eq!(__muldi3(i64::MAX, 2), i64::MAX.wrapping_mul(2));
        assert_eq!(__muldi3(i64::MIN, -1), i64::MIN.wrapping_mul(-1));
        for u in [
            i64::MIN,
            -0x7fff_ffff_ffff,
            -3,
            -1,
            0,
            1,
            3,
            0x7fff_ffff,
            i64::MAX,
        ] {
            for v in [i64::MIN, -0x1234_5678, -1, 0, 1, 0x1234_5678, i64::MAX] {
                assert_eq!(muldi3_by_words(u, v), u.wrapping_mul(v));
                assert_eq!(__muldi3(u, v), u.wrapping_mul(v));
            }
        }
    }

    #[test]
    fn muldi3_export_registers_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("__muldi3"),
            Some(__muldi3 as usize)
        );
    }
}
