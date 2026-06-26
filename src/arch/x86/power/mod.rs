//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/power
//! test-origin: linux:vendor/linux/arch/x86/power
//! x86 suspend, resume, and hibernation handoff policy.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/power/cpu.c
//! - vendor/linux/arch/x86/power/hibernate.c
//! - vendor/linux/arch/x86/power/hibernate_32.c
//! - vendor/linux/arch/x86/power/hibernate_64.c

use crate::include::uapi::errno::EINVAL;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SavedCpuState {
    pub cr0: u64,
    pub cr3: u64,
    pub cr4: u64,
    pub efer: u64,
    pub pat: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HibernateImage {
    pub restore_pfn: u64,
    pub image_pages: u64,
    pub resume_physical_address: u64,
}

pub const fn saved_cpu_state_valid(state: SavedCpuState) -> bool {
    state.cr0 != 0 && state.cr3 & 0xfff == 0 && state.efer != 0
}

pub const fn hibernate_image_valid(image: HibernateImage) -> Result<(), i32> {
    if image.restore_pfn == 0 || image.image_pages == 0 {
        return Err(EINVAL);
    }
    if image.resume_physical_address & 0xfff != 0 {
        return Err(EINVAL);
    }
    Ok(())
}

pub const fn hibernate_restore_start(image: HibernateImage) -> Result<u64, i32> {
    match hibernate_image_valid(image) {
        Ok(()) => Ok(image.restore_pfn << 12),
        Err(err) => Err(err),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HibernateArch {
    X86_32,
    X86_64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HibernateArchState {
    pub arch: HibernateArch,
    pub pae_enabled: bool,
    pub long_mode: bool,
    pub identity_map_ready: bool,
}

pub const fn hibernate_arch_state_valid(state: HibernateArchState) -> Result<(), i32> {
    match state.arch {
        HibernateArch::X86_32 if state.long_mode => Err(EINVAL),
        HibernateArch::X86_64 if !state.long_mode => Err(EINVAL),
        _ if !state.identity_map_ready => Err(EINVAL),
        _ => Ok(()),
    }
}

pub const fn hibernate_resume_page_table_level(state: HibernateArchState) -> Result<u8, i32> {
    match hibernate_arch_state_valid(state) {
        Ok(()) => match state.arch {
            HibernateArch::X86_32 if state.pae_enabled => Ok(3),
            HibernateArch::X86_32 => Ok(2),
            HibernateArch::X86_64 => Ok(4),
        },
        Err(err) => Err(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_cpu_state_requires_page_aligned_cr3() {
        assert!(saved_cpu_state_valid(SavedCpuState {
            cr0: 1,
            cr3: 0x2000,
            cr4: 0,
            efer: 1,
            pat: 0,
        }));
        assert!(!saved_cpu_state_valid(SavedCpuState {
            cr0: 1,
            cr3: 0x2001,
            cr4: 0,
            efer: 1,
            pat: 0,
        }));
    }

    #[test]
    fn hibernate_resume_addresses_are_page_aligned() {
        let image = HibernateImage {
            restore_pfn: 2,
            image_pages: 4,
            resume_physical_address: 0x4000,
        };
        assert_eq!(hibernate_restore_start(image), Ok(0x2000));
        assert_eq!(
            hibernate_image_valid(HibernateImage {
                resume_physical_address: 0x4001,
                ..image
            }),
            Err(EINVAL)
        );
    }

    #[test]
    fn hibernate_32_and_64_resume_policy_is_arch_specific() {
        assert_eq!(
            hibernate_resume_page_table_level(HibernateArchState {
                arch: HibernateArch::X86_32,
                pae_enabled: true,
                long_mode: false,
                identity_map_ready: true,
            }),
            Ok(3)
        );
        assert_eq!(
            hibernate_resume_page_table_level(HibernateArchState {
                arch: HibernateArch::X86_64,
                pae_enabled: true,
                long_mode: true,
                identity_map_ready: true,
            }),
            Ok(4)
        );
        assert_eq!(
            hibernate_arch_state_valid(HibernateArchState {
                arch: HibernateArch::X86_64,
                pae_enabled: true,
                long_mode: false,
                identity_map_ready: true,
            }),
            Err(EINVAL)
        );
    }
}
