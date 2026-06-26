//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kvm/smm.c
//! test-origin: linux:vendor/linux/arch/x86/kvm/smm.c
//! KVM System Management Mode (SMM) emulation.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kvm/smm.c

// When the guest receives an SMI, KVM saves CPU state into the SMM
// save area (256 bytes on 32-bit, 512 bytes on 64-bit). We model the
// save-area layout shape; full save/restore is gated behind the
// virtualization runtime.

pub const SMM_SAVE_AREA_32: usize = 256;
pub const SMM_SAVE_AREA_64: usize = 512;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SmmMode {
    Legacy32,
    Long64,
}

pub const fn save_area_size(mode: SmmMode) -> usize {
    match mode {
        SmmMode::Legacy32 => SMM_SAVE_AREA_32,
        SmmMode::Long64 => SMM_SAVE_AREA_64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_area_size_matches_mode() {
        assert_eq!(save_area_size(SmmMode::Legacy32), 256);
        assert_eq!(save_area_size(SmmMode::Long64), 512);
    }
}
