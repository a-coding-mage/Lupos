//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/mtrr/cyrix.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/mtrr/cyrix.c
//! Cyrix Address Region Registers (ARR) MTRR vendor operations.
//!
//! The real Linux code programs Cyrix configuration registers through ports
//! 0x22/0x23. This module models the register bytes and region-selection
//! policy without doing port I/O.

use crate::include::uapi::errno::{EINVAL, ENOSPC};

pub const PAGE_SHIFT: u8 = 12;
pub const CYRIX_ARR_COUNT: usize = 8;
pub const CX86_CCR3: u8 = 0xc3;
pub const CX86_ARR_BASE: u8 = 0xc4;
pub const CX86_RCR_BASE: u8 = 0xdc;
pub const CX86_CCR3_MAPEN: u8 = 0x10;

pub const MTRR_TYPE_UNCACHABLE: u8 = 0;
pub const MTRR_TYPE_WRCOMB: u8 = 1;
pub const MTRR_TYPE_WRTHROUGH: u8 = 4;
pub const MTRR_TYPE_WRBACK: u8 = 6;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CyrixArrRegisters {
    pub base_hi: u8,
    pub base_mid: u8,
    pub base_low_and_size: u8,
    pub rcr: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CyrixDecodedArr {
    pub base_pages: u64,
    pub size_pages: u64,
    pub ty: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CyrixSetArrPlan {
    pub arr_index: u8,
    pub base_hi: u8,
    pub base_mid: u8,
    pub base_low_and_size: u8,
    pub rcr_index: u8,
    pub rcr_value: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CyrixMtrrOps {
    pub var_regs: u32,
    pub set: &'static str,
    pub get: &'static str,
    pub get_free_region: &'static str,
    pub validate_add_page: &'static str,
    pub have_wrcomb: &'static str,
}

pub const CYRIX_MTRR_OPS: CyrixMtrrOps = CyrixMtrrOps {
    var_regs: 8,
    set: "cyrix_set_arr",
    get: "cyrix_get_arr",
    get_free_region: "cyrix_get_free_region",
    validate_add_page: "generic_validate_add_page",
    have_wrcomb: "positive_have_wrcomb",
};

pub const fn cyrix_arr_index(reg: usize) -> Result<u8, i32> {
    if reg >= CYRIX_ARR_COUNT {
        return Err(-EINVAL);
    }
    Ok(CX86_ARR_BASE + ((reg as u8) << 1) + reg as u8)
}

pub const fn cyrix_rcr_index(reg: usize) -> Result<u8, i32> {
    if reg >= CYRIX_ARR_COUNT {
        return Err(-EINVAL);
    }
    Ok(CX86_RCR_BASE + reg as u8)
}

pub const fn cyrix_type_from_rcr(reg: usize, rcr: u8) -> u8 {
    if reg < 7 {
        match rcr {
            1 => MTRR_TYPE_UNCACHABLE,
            8 => MTRR_TYPE_WRBACK,
            9 => MTRR_TYPE_WRCOMB,
            24 => MTRR_TYPE_WRTHROUGH,
            _ => MTRR_TYPE_WRTHROUGH,
        }
    } else {
        match rcr {
            0 => MTRR_TYPE_UNCACHABLE,
            8 => MTRR_TYPE_WRCOMB,
            9 => MTRR_TYPE_WRBACK,
            25 => MTRR_TYPE_WRTHROUGH,
            _ => MTRR_TYPE_WRTHROUGH,
        }
    }
}

pub const fn cyrix_rcr_from_type(reg: usize, ty: u8) -> u8 {
    if reg < 7 {
        match ty {
            MTRR_TYPE_UNCACHABLE => 1,
            MTRR_TYPE_WRCOMB => 9,
            MTRR_TYPE_WRTHROUGH => 24,
            _ => 8,
        }
    } else {
        match ty {
            MTRR_TYPE_UNCACHABLE => 0,
            MTRR_TYPE_WRCOMB => 8,
            MTRR_TYPE_WRTHROUGH => 25,
            _ => 9,
        }
    }
}

pub const fn cyrix_arr_size_code(reg: usize, mut size_pages: u64) -> u8 {
    if reg >= 7 {
        size_pages >>= 6;
    }
    size_pages &= 0x7fff;

    let mut arr_size = 0u8;
    while size_pages != 0 {
        arr_size += 1;
        size_pages >>= 1;
    }
    arr_size
}

pub const fn cyrix_encode_base_bytes(base_pages: u64, arr_size: u8) -> [u8; 3] {
    let base = base_pages << PAGE_SHIFT;
    [
        ((base >> 24) & 0xff) as u8,
        ((base >> 16) & 0xff) as u8,
        (((base >> 8) & 0xf0) as u8) | (arr_size & 0x0f),
    ]
}

pub const fn cyrix_decode_arr(reg: usize, raw: CyrixArrRegisters) -> Result<CyrixDecodedArr, i32> {
    if reg >= CYRIX_ARR_COUNT {
        return Err(-EINVAL);
    }
    let shift = raw.base_low_and_size & 0x0f;
    let base = ((raw.base_hi as u64) << 24)
        | ((raw.base_mid as u64) << 16)
        | ((raw.base_low_and_size as u64) << 8);
    let size_pages = if shift != 0 {
        (if reg < 7 { 0x1u64 } else { 0x40u64 }) << (shift - 1)
    } else {
        0
    };
    Ok(CyrixDecodedArr {
        base_pages: base >> PAGE_SHIFT,
        size_pages,
        ty: cyrix_type_from_rcr(reg, raw.rcr),
    })
}

pub const fn cyrix_set_arr_plan(
    reg: usize,
    base_pages: u64,
    size_pages: u64,
    ty: u8,
) -> Result<CyrixSetArrPlan, i32> {
    let arr_index = match cyrix_arr_index(reg) {
        Ok(index) => index,
        Err(err) => return Err(err),
    };
    let rcr_index = match cyrix_rcr_index(reg) {
        Ok(index) => index,
        Err(err) => return Err(err),
    };
    let arr_size = cyrix_arr_size_code(reg, size_pages);
    let base = cyrix_encode_base_bytes(base_pages, arr_size);
    Ok(CyrixSetArrPlan {
        arr_index,
        base_hi: base[0],
        base_mid: base[1],
        base_low_and_size: base[2],
        rcr_index,
        rcr_value: cyrix_rcr_from_type(reg, ty),
    })
}

pub fn cyrix_get_free_region(
    size_pages: u64,
    replace_reg: i32,
    arrs: &[CyrixDecodedArr; CYRIX_ARR_COUNT],
) -> Result<usize, i32> {
    match replace_reg {
        7 if size_pages >= 0x40 => return Ok(7),
        4..=6 => return Ok(replace_reg as usize),
        0..=3 => return Ok(replace_reg as usize),
        _ => {}
    }

    if size_pages > 0x2000 {
        if arrs[7].size_pages == 0 {
            return Ok(7);
        }
    } else {
        for (i, arr) in arrs.iter().take(7).enumerate() {
            if arr.size_pages == 0 {
                return Ok(i);
            }
        }
        if arrs[7].size_pages == 0 && size_pages >= 0x40 {
            return Ok(7);
        }
    }
    Err(-ENOSPC)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cyrix_source_contract_matches_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kernel/cpu/mtrr/cyrix.c"
        ));
        let flags = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/include/uapi/asm/processor-flags.h"
        ));
        let uapi = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/include/uapi/asm/mtrr.h"
        ));
        let local = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kernel/cpu/mtrr/mtrr.h"
        ));

        assert!(source.contains("arr = CX86_ARR_BASE + (reg << 1) + reg"));
        assert!(source.contains("setCx86(CX86_CCR3, (ccr3 & 0x0f) | 0x10);"));
        assert!(source.contains("case 1:"));
        assert!(source.contains("*type = MTRR_TYPE_UNCACHABLE;"));
        assert!(source.contains("case 25:"));
        assert!(source.contains("*type = MTRR_TYPE_WRTHROUGH;"));
        assert!(source.contains("if (size > 0x2000)"));
        assert!(source.contains("if ((lsize == 0) && (size >= 0x40))"));
        assert!(source.contains("setCx86(CX86_RCR_BASE + reg, arr_type);"));
        assert!(source.contains("const struct mtrr_ops cyrix_mtrr_ops"));
        assert!(flags.contains("#define CX86_CCR3\t0xc3"));
        assert!(flags.contains("#define CX86_ARR_BASE\t0xc4"));
        assert!(flags.contains("#define CX86_RCR_BASE\t0xdc"));
        assert!(uapi.contains("#define MTRR_TYPE_WRTHROUGH  4"));
        assert!(local.contains("struct mtrr_ops"));

        assert_eq!(CYRIX_MTRR_OPS.var_regs, 8);
        assert_eq!(
            CYRIX_MTRR_OPS.validate_add_page,
            "generic_validate_add_page"
        );
        assert_eq!(CYRIX_MTRR_OPS.have_wrcomb, "positive_have_wrcomb");
    }

    #[test]
    fn cyrix_arr_decode_and_set_plan_match_register_layout() {
        let plan = cyrix_set_arr_plan(0, 0x12345, 0x20, MTRR_TYPE_WRCOMB).unwrap();
        assert_eq!(plan.arr_index, CX86_ARR_BASE);
        assert_eq!(plan.rcr_index, CX86_RCR_BASE);
        assert_eq!(plan.rcr_value, 9);
        assert_eq!(plan.base_hi, 0x12);
        assert_eq!(plan.base_mid, 0x34);
        assert_eq!(plan.base_low_and_size & 0xf0, 0x50);
        assert_eq!(plan.base_low_and_size & 0x0f, 6);

        let decoded = cyrix_decode_arr(
            0,
            CyrixArrRegisters {
                base_hi: plan.base_hi,
                base_mid: plan.base_mid,
                base_low_and_size: plan.base_low_and_size,
                rcr: plan.rcr_value,
            },
        )
        .unwrap();
        assert_eq!(decoded.base_pages, 0x12345);
        assert_eq!(decoded.size_pages, 0x20);
        assert_eq!(decoded.ty, MTRR_TYPE_WRCOMB);

        let arr7 = cyrix_set_arr_plan(7, 0x80000, 0x40, MTRR_TYPE_WRBACK).unwrap();
        assert_eq!(arr7.rcr_value, 9);
        assert_eq!(arr7.base_hi, 0x80);
        assert_eq!(arr7.base_low_and_size & 0x0f, 1);
        assert_eq!(
            cyrix_type_from_rcr(7, 8),
            MTRR_TYPE_WRCOMB,
            "ARR7 bit 0 has inverted cache-enable meaning"
        );
    }

    #[test]
    fn cyrix_free_region_tracks_arr7_size_rules() {
        let used = CyrixDecodedArr {
            base_pages: 0,
            size_pages: 1,
            ty: MTRR_TYPE_WRBACK,
        };
        let free = CyrixDecodedArr {
            base_pages: 0,
            size_pages: 0,
            ty: MTRR_TYPE_UNCACHABLE,
        };
        let mut arrs = [used; CYRIX_ARR_COUNT];
        arrs[3] = free;
        assert_eq!(cyrix_get_free_region(0x20, -1, &arrs), Ok(3));
        assert_eq!(cyrix_get_free_region(0x20, 5, &arrs), Ok(5));
        assert_eq!(cyrix_get_free_region(0x20, 7, &arrs), Ok(3));

        arrs = [used; CYRIX_ARR_COUNT];
        arrs[7] = free;
        assert_eq!(cyrix_get_free_region(0x20, -1, &arrs), Err(-ENOSPC));
        assert_eq!(cyrix_get_free_region(0x40, -1, &arrs), Ok(7));
        assert_eq!(cyrix_get_free_region(0x2001, -1, &arrs), Ok(7));
        arrs[7] = used;
        assert_eq!(cyrix_get_free_region(0x2001, -1, &arrs), Err(-ENOSPC));
    }
}
