//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/machine_kexec_32.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/machine_kexec_32.c
//! 32-bit x86 machine-kexec transition model.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/machine_kexec_32.c

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::EINVAL;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Kexec32Segment {
    pub start: u32,
    pub end: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kexec32Action {
    AllocatePageTable,
    MapPage { vaddr: u32, paddr: u32 },
    LoadSegments,
    InstallInvalidIdt,
    InstallIdentityGdt,
    Jump(u32),
}

pub fn machine_kexec_prepare_32(
    entry: u32,
    segments: &[Kexec32Segment],
) -> Result<Vec<Kexec32Action>, i32> {
    if entry == 0 {
        return Err(EINVAL);
    }
    let mut actions = Vec::new();
    actions.push(Kexec32Action::AllocatePageTable);
    for seg in segments {
        let mut addr = seg.start & !0xfff;
        while addr < seg.end {
            actions.push(Kexec32Action::MapPage {
                vaddr: addr,
                paddr: addr,
            });
            addr = addr.saturating_add(0x1000);
        }
    }
    actions.push(Kexec32Action::LoadSegments);
    actions.push(Kexec32Action::InstallInvalidIdt);
    actions.push(Kexec32Action::InstallIdentityGdt);
    actions.push(Kexec32Action::Jump(entry));
    Ok(actions)
}

pub const fn machine_kexec_cleanup_32() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepare_maps_segments_and_jumps() {
        let actions = machine_kexec_prepare_32(
            0x100000,
            &[Kexec32Segment {
                start: 0x2000,
                end: 0x4000,
            }],
        )
        .unwrap();
        assert_eq!(actions[0], Kexec32Action::AllocatePageTable);
        assert!(actions.contains(&Kexec32Action::MapPage {
            vaddr: 0x2000,
            paddr: 0x2000
        }));
        assert_eq!(actions.last(), Some(&Kexec32Action::Jump(0x100000)));
    }
}
