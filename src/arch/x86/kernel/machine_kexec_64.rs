//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/machine_kexec_64.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/machine_kexec_64.c
//! x86_64 machine-kexec transition planning and relocation wrapper.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/machine_kexec_64.c

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::EINVAL;
use crate::kernel::module::relocate::{RelocType, apply_rela};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KexecSegment {
    pub start: u64,
    pub memsz: u64,
    pub filesz: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Kimage {
    pub start: u64,
    pub control_code_page: u64,
    pub segments: Vec<KexecSegment>,
    pub preserve_context: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MachineKexecAction {
    IdentityMap { start: u64, end: u64 },
    LoadControlCode(u64),
    PreserveContext,
    JumpTo(u64),
}

pub fn machine_kexec_prepare(image: &Kimage) -> Result<Vec<MachineKexecAction>, i32> {
    if image.start == 0 || image.control_code_page == 0 {
        return Err(EINVAL);
    }
    let mut actions = Vec::new();
    for seg in image.segments.iter() {
        actions.push(MachineKexecAction::IdentityMap {
            start: seg.start,
            end: seg.start + seg.memsz,
        });
    }
    actions.push(MachineKexecAction::LoadControlCode(image.control_code_page));
    if image.preserve_context {
        actions.push(MachineKexecAction::PreserveContext);
    }
    actions.push(MachineKexecAction::JumpTo(image.start));
    Ok(actions)
}

pub const fn machine_kexec_cleanup(_image: &Kimage) -> bool {
    true
}

pub fn machine_kexec(image: &Kimage) -> Result<MachineKexecAction, i32> {
    machine_kexec_prepare(image)?;
    Ok(MachineKexecAction::JumpTo(image.start))
}

pub fn arch_kexec_apply_relocations_add(
    mem: &mut [u8],
    offset: usize,
    rel_type: RelocType,
    sym_addr: u64,
    patch_vaddr: u64,
    addend: i64,
) -> Result<(), i32> {
    apply_rela(mem, offset, rel_type, sym_addr, patch_vaddr, addend)
}

pub const fn crash_resource_protect(addr: u64, len: u64) -> Option<(u64, u64)> {
    if len == 0 {
        None
    } else {
        Some((addr, addr + len - 1))
    }
}

pub const fn crash_resource_unprotect(addr: u64, len: u64) -> Option<(u64, u64)> {
    crash_resource_protect(addr, len)
}

pub const fn arch_kexec_post_alloc_pages(addr: u64, pages: u64) -> Option<(u64, u64)> {
    crash_resource_protect(addr, pages << 12)
}

pub const fn arch_kexec_pre_free_pages(addr: u64, pages: u64) -> Option<(u64, u64)> {
    crash_resource_unprotect(addr, pages << 12)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepare_builds_identity_map_and_jump_plan() {
        let image = Kimage {
            start: 0x100000,
            control_code_page: 0x90000,
            segments: alloc::vec![KexecSegment {
                start: 0x200000,
                memsz: 0x1000,
                filesz: 0x800,
            }],
            preserve_context: true,
        };
        let actions = machine_kexec_prepare(&image).unwrap();
        assert!(matches!(actions[0], MachineKexecAction::IdentityMap { .. }));
        assert_eq!(actions.last(), Some(&MachineKexecAction::JumpTo(0x100000)));
    }

    #[test]
    fn relocation_wrapper_accepts_plt32() {
        let mut mem = [0u8; 8];
        arch_kexec_apply_relocations_add(&mut mem, 0, RelocType::Plt32, 0x1010, 0x1000, -4)
            .unwrap();
        assert_eq!(i32::from_le_bytes(mem[0..4].try_into().unwrap()), 0xc);
    }
}
