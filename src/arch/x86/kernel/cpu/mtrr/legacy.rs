//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/mtrr/legacy.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/mtrr/legacy.c
//! Legacy 32-bit MTRR vendor selection and syscore save/restore hooks.

use crate::include::uapi::errno::{EINVAL, ENOMEM};

pub const LEGACY_MTRR_VAR_COUNT: u8 = 8;
pub const MSR_IA32_MTRR_PHYSBASE0: u32 = 0x0000_0200;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X86Vendor {
    Amd,
    Centaur,
    Cyrix,
    Other,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LegacyMtrrFeatures {
    pub k6_mtrr: bool,
    pub centaur_mcr: bool,
    pub cyrix_arr: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LegacyMtrrOps {
    Amd,
    Centaur,
    Cyrix,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MtrrValue {
    pub ltype: u8,
    pub lbase: u64,
    pub lsize: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MtrrSetCall {
    pub reg: usize,
    pub base: u64,
    pub size: u64,
    pub ty: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MtrrSyscoreState {
    pub mtrr_value: Option<alloc::vec::Vec<MtrrValue>>,
    pub registered: bool,
}

extern crate alloc;

pub const fn physbase_msr(index: u8) -> Result<u32, i32> {
    if index >= LEGACY_MTRR_VAR_COUNT {
        return Err(EINVAL);
    }
    Ok(MSR_IA32_MTRR_PHYSBASE0 + (index as u32) * 2)
}

pub const fn physmask_msr(index: u8) -> Result<u32, i32> {
    if index >= LEGACY_MTRR_VAR_COUNT {
        return Err(EINVAL);
    }
    Ok(MSR_IA32_MTRR_PHYSBASE0 + (index as u32) * 2 + 1)
}

pub const fn mtrr_set_if(vendor: X86Vendor, features: LegacyMtrrFeatures) -> Option<LegacyMtrrOps> {
    match vendor {
        X86Vendor::Amd if features.k6_mtrr => Some(LegacyMtrrOps::Amd),
        X86Vendor::Centaur if features.centaur_mcr => Some(LegacyMtrrOps::Centaur),
        X86Vendor::Cyrix if features.cyrix_arr => Some(LegacyMtrrOps::Cyrix),
        _ => None,
    }
}

pub fn mtrr_save(saved: Option<&mut [MtrrValue]>, current_ranges: &[MtrrValue]) -> Result<(), i32> {
    let saved = saved.ok_or(-ENOMEM)?;
    for (dst, src) in saved.iter_mut().zip(current_ranges.iter()) {
        *dst = *src;
    }
    Ok(())
}

pub fn mtrr_restore(saved: &[MtrrValue]) -> alloc::vec::Vec<MtrrSetCall> {
    let mut calls = alloc::vec::Vec::new();
    for (reg, value) in saved.iter().enumerate() {
        if value.lsize != 0 {
            calls.push(MtrrSetCall {
                reg,
                base: value.lbase,
                size: value.lsize,
                ty: value.ltype,
            });
        }
    }
    calls
}

pub fn mtrr_register_syscore(
    num_var_ranges: usize,
    allocation_available: bool,
) -> MtrrSyscoreState {
    MtrrSyscoreState {
        mtrr_value: if allocation_available {
            Some(alloc::vec![MtrrValue::default(); num_var_ranges])
        } else {
            None
        },
        registered: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_mtrr_source_contract_matches_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kernel/cpu/mtrr/legacy.c"
        ));
        assert!(source.contains("void mtrr_set_if(void)"));
        assert!(source.contains("case X86_VENDOR_AMD:"));
        assert!(source.contains("X86_FEATURE_K6_MTRR"));
        assert!(source.contains("case X86_VENDOR_CENTAUR:"));
        assert!(source.contains("X86_FEATURE_CENTAUR_MCR"));
        assert!(source.contains("case X86_VENDOR_CYRIX:"));
        assert!(source.contains("X86_FEATURE_CYRIX_ARR"));
        assert!(source.contains("static int mtrr_save(void *data)"));
        assert!(source.contains("if (!mtrr_value)"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("mtrr_if->get(i, &mtrr_value[i].lbase"));
        assert!(source.contains("static void mtrr_restore(void *data)"));
        assert!(source.contains("if (mtrr_value[i].lsize)"));
        assert!(source.contains("mtrr_if->set(i, mtrr_value[i].lbase"));
        assert!(source.contains("register_syscore(&mtrr_syscore);"));

        assert_eq!(physbase_msr(0), Ok(0x200));
        assert_eq!(physmask_msr(0), Ok(0x201));
        assert_eq!(physbase_msr(7), Ok(0x20e));
        assert_eq!(physbase_msr(8), Err(EINVAL));
    }

    #[test]
    fn vendor_selection_matches_legacy_switch() {
        assert_eq!(
            mtrr_set_if(
                X86Vendor::Amd,
                LegacyMtrrFeatures {
                    k6_mtrr: true,
                    ..LegacyMtrrFeatures::default()
                }
            ),
            Some(LegacyMtrrOps::Amd)
        );
        assert_eq!(
            mtrr_set_if(
                X86Vendor::Centaur,
                LegacyMtrrFeatures {
                    centaur_mcr: true,
                    ..LegacyMtrrFeatures::default()
                }
            ),
            Some(LegacyMtrrOps::Centaur)
        );
        assert_eq!(
            mtrr_set_if(
                X86Vendor::Cyrix,
                LegacyMtrrFeatures {
                    cyrix_arr: true,
                    ..LegacyMtrrFeatures::default()
                }
            ),
            Some(LegacyMtrrOps::Cyrix)
        );
        assert_eq!(
            mtrr_set_if(X86Vendor::Amd, LegacyMtrrFeatures::default()),
            None
        );
        assert_eq!(
            mtrr_set_if(
                X86Vendor::Other,
                LegacyMtrrFeatures {
                    k6_mtrr: true,
                    centaur_mcr: true,
                    cyrix_arr: true,
                }
            ),
            None
        );
    }

    #[test]
    fn save_restore_and_syscore_registration_match_linux_edges() {
        let current = [
            MtrrValue {
                ltype: 1,
                lbase: 0x1000,
                lsize: 0x2000,
            },
            MtrrValue {
                ltype: 2,
                lbase: 0x4000,
                lsize: 0,
            },
        ];
        let mut saved = [MtrrValue::default(); 2];
        assert_eq!(mtrr_save(Some(&mut saved), &current), Ok(()));
        assert_eq!(saved, current);
        assert_eq!(mtrr_save(None, &current), Err(-ENOMEM));

        assert_eq!(
            mtrr_restore(&saved),
            alloc::vec![MtrrSetCall {
                reg: 0,
                base: 0x1000,
                size: 0x2000,
                ty: 1,
            }]
        );

        let registered = mtrr_register_syscore(3, true);
        assert!(registered.registered);
        assert_eq!(registered.mtrr_value.unwrap().len(), 3);
        let no_allocation = mtrr_register_syscore(3, false);
        assert!(no_allocation.registered);
        assert!(no_allocation.mtrr_value.is_none());
    }
}
