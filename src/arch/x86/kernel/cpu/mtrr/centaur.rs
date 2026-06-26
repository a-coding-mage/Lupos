//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/mtrr/centaur.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/mtrr/centaur.c
//! Centaur/VIA MTRR vendor-specific register support.

extern crate alloc;

use crate::include::uapi::errno::{EINVAL, ENOSPC};

pub const MSR_CENTAUR_MCR0: u32 = 0x0000_0110;
pub const MSR_CENTAUR_MCR_COUNT: u8 = 8;
pub const PAGE_SHIFT: u32 = 12;
pub const MTRR_TYPE_UNCACHABLE: u8 = 0;
pub const MTRR_TYPE_WRCOMB: u8 = 1;
pub const MTRR_TYPE_WRBACK: u8 = 6;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CentaurMcr {
    pub high: u64,
    pub low: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CentaurDecodedMcr {
    pub base: u64,
    pub size: u64,
    pub ty: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CentaurWrmsr {
    pub msr: u32,
    pub low: u64,
    pub high: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CentaurMtrrOps {
    pub var_regs: u8,
    pub set: &'static str,
    pub get: &'static str,
    pub get_free_region: &'static str,
    pub validate_add_page: &'static str,
    pub have_wrcomb: &'static str,
}

pub const CENTAUR_MTRR_OPS: CentaurMtrrOps = CentaurMtrrOps {
    var_regs: 8,
    set: "centaur_set_mcr",
    get: "centaur_get_mcr",
    get_free_region: "centaur_get_free_region",
    validate_add_page: "centaur_validate_add_page",
    have_wrcomb: "positive_have_wrcomb",
};

pub const fn index_msr(index: usize) -> Option<u32> {
    if index < MSR_CENTAUR_MCR_COUNT as usize {
        Some(MSR_CENTAUR_MCR0 + index as u32)
    } else {
        None
    }
}

pub fn centaur_get_free_region(
    num_var_ranges: usize,
    replace_reg: i32,
    reserved: u8,
    mcr_type: u8,
    mcrs: &[CentaurMcr],
) -> Result<usize, i32> {
    let max = num_var_ranges.min(MSR_CENTAUR_MCR_COUNT as usize);
    if replace_reg >= 0 && (replace_reg as usize) < max {
        return Ok(replace_reg as usize);
    }

    for i in 0..max {
        if reserved & (1 << i) != 0 {
            continue;
        }
        if centaur_get_mcr(mcr_type, i, mcrs).is_some_and(|mcr| mcr.size == 0) {
            return Ok(i);
        }
    }

    Err(-ENOSPC)
}

pub fn centaur_get_mcr(mcr_type: u8, reg: usize, mcrs: &[CentaurMcr]) -> Option<CentaurDecodedMcr> {
    let mcr = *mcrs.get(reg)?;
    let low_type = mcr.low & 31;
    let mut ty = MTRR_TYPE_WRCOMB;
    if mcr_type == 1 && low_type & 2 != 0 {
        ty = MTRR_TYPE_UNCACHABLE;
    }
    if mcr_type == 1 && low_type == 25 {
        ty = MTRR_TYPE_WRBACK;
    }
    if mcr_type == 0 && low_type == 31 {
        ty = MTRR_TYPE_WRBACK;
    }
    Some(CentaurDecodedMcr {
        base: mcr.high >> PAGE_SHIFT,
        size: u32::wrapping_neg((mcr.low as u32) & 0xffff_f000) as u64 >> PAGE_SHIFT,
        ty,
    })
}

pub fn centaur_set_mcr(
    mcr_type: u8,
    reg: usize,
    base: u64,
    size: u64,
    ty: u8,
    mcrs: &mut [CentaurMcr],
) -> Option<CentaurWrmsr> {
    let slot = mcrs.get_mut(reg)?;
    let (low, high) = if size == 0 {
        (0, 0)
    } else {
        let high = base << PAGE_SHIFT;
        let encoded_size = (u32::wrapping_neg(size as u32) << PAGE_SHIFT) as u64;
        let low = if mcr_type == 0 {
            encoded_size | 0x1f
        } else if ty == MTRR_TYPE_UNCACHABLE {
            encoded_size | 0x02
        } else {
            encoded_size | 0x09
        };
        (low, high)
    };
    *slot = CentaurMcr { high, low };
    Some(CentaurWrmsr {
        msr: index_msr(reg)?,
        low,
        high,
    })
}

pub const fn centaur_validate_add_page(mcr_type: u8, ty: u8) -> Result<(), i32> {
    if ty != MTRR_TYPE_WRCOMB && (mcr_type == 0 || ty != MTRR_TYPE_UNCACHABLE) {
        Err(-EINVAL)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centaur_mtrr_source_contract_matches_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kernel/cpu/mtrr/centaur.c"
        ));
        let uapi = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/include/uapi/asm/mtrr.h"
        ));
        let msr_index = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/include/asm/msr-index.h"
        ));
        assert!(source.contains("static struct {"));
        assert!(source.contains("centaur_mcr[8];"));
        assert!(source.contains("static u8 centaur_mcr_reserved;"));
        assert!(source.contains("static u8 centaur_mcr_type;"));
        assert!(source.contains("centaur_get_free_region"));
        assert!(source.contains("if (replace_reg >= 0 && replace_reg < max)"));
        assert!(source.contains("centaur_mcr_reserved & (1 << i)"));
        assert!(source.contains("return -ENOSPC;"));
        assert!(source.contains("centaur_get_mcr"));
        assert!(source.contains("*base = centaur_mcr[reg].high >> PAGE_SHIFT;"));
        assert!(source.contains("*size = -(centaur_mcr[reg].low & 0xfffff000) >> PAGE_SHIFT;"));
        assert!(source.contains("centaur_set_mcr"));
        assert!(source.contains("wrmsr(MSR_IDT_MCR0 + reg, low, high);"));
        assert!(source.contains("centaur_validate_add_page"));
        assert!(source.contains("const struct mtrr_ops centaur_mtrr_ops"));
        assert!(uapi.contains("#define MTRR_TYPE_UNCACHABLE 0"));
        assert!(uapi.contains("#define MTRR_TYPE_WRCOMB     1"));
        assert!(uapi.contains("#define MTRR_TYPE_WRBACK     6"));
        assert!(msr_index.contains("#define MSR_IDT_MCR0"));

        assert_eq!(CENTAUR_MTRR_OPS.var_regs, 8);
        assert_eq!(CENTAUR_MTRR_OPS.have_wrcomb, "positive_have_wrcomb");
        assert_eq!(index_msr(0), Some(0x110));
        assert_eq!(index_msr(7), Some(0x117));
        assert_eq!(index_msr(8), None);
    }

    #[test]
    fn get_free_region_respects_replacement_reserved_and_empty_slots() {
        let mcrs = [
            CentaurMcr {
                high: 0,
                low: 0xffff_0001,
            },
            CentaurMcr { high: 0, low: 0 },
            CentaurMcr { high: 0, low: 0 },
        ];
        assert_eq!(centaur_get_free_region(3, 2, 0, 0, &mcrs), Ok(2));
        assert_eq!(centaur_get_free_region(3, -1, 0b0000_0010, 0, &mcrs), Ok(2));
        assert_eq!(
            centaur_get_free_region(1, -1, 0b0000_0001, 0, &mcrs),
            Err(-ENOSPC)
        );
    }

    #[test]
    fn get_and_set_mcr_match_centaur_type_edges() {
        let mut mcrs = [CentaurMcr { high: 0, low: 0 }; 8];
        let wrmsr = centaur_set_mcr(0, 0, 0x20, 0x10, MTRR_TYPE_WRCOMB, &mut mcrs).unwrap();
        assert_eq!(
            wrmsr,
            CentaurWrmsr {
                msr: 0x110,
                low: (u32::wrapping_neg(0x10) << PAGE_SHIFT) as u64 | 0x1f,
                high: 0x20 << PAGE_SHIFT,
            }
        );
        assert_eq!(
            centaur_get_mcr(0, 0, &mcrs),
            Some(CentaurDecodedMcr {
                base: 0x20,
                size: 0x10,
                ty: MTRR_TYPE_WRBACK,
            })
        );

        centaur_set_mcr(1, 1, 0x30, 0x20, MTRR_TYPE_UNCACHABLE, &mut mcrs).unwrap();
        assert_eq!(
            centaur_get_mcr(1, 1, &mcrs).unwrap().ty,
            MTRR_TYPE_UNCACHABLE
        );
        centaur_set_mcr(1, 2, 0x40, 0x20, MTRR_TYPE_WRCOMB, &mut mcrs).unwrap();
        assert_eq!(centaur_get_mcr(1, 2, &mcrs).unwrap().ty, MTRR_TYPE_WRCOMB);
        centaur_set_mcr(1, 3, 0, 0, MTRR_TYPE_WRCOMB, &mut mcrs).unwrap();
        assert_eq!(mcrs[3], CentaurMcr { high: 0, low: 0 });
    }

    #[test]
    fn validate_add_page_allows_linux_supported_types() {
        assert_eq!(centaur_validate_add_page(0, MTRR_TYPE_WRCOMB), Ok(()));
        assert_eq!(
            centaur_validate_add_page(0, MTRR_TYPE_UNCACHABLE),
            Err(-EINVAL)
        );
        assert_eq!(centaur_validate_add_page(1, MTRR_TYPE_UNCACHABLE), Ok(()));
        assert_eq!(centaur_validate_add_page(1, MTRR_TYPE_WRBACK), Err(-EINVAL));
    }
}
