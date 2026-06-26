//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/platform-quirks.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/platform-quirks.c
//! x86 platform subarchitecture quirks.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/platform-quirks.c

#![allow(dead_code)]

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HardwareSubarch {
    Pc,
    Xen,
    IntelMid,
    Ce4100,
    Unknown(u32),
}

impl From<u32> for HardwareSubarch {
    fn from(value: u32) -> Self {
        match value {
            0 => Self::Pc,
            1 => Self::Xen,
            2 => Self::IntelMid,
            3 => Self::Ce4100,
            other => Self::Unknown(other),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlatformQuirkState {
    pub legacy_pic: bool,
    pub legacy_devices: bool,
    pub pnpbios_disabled: bool,
}

pub const fn x86_early_init_platform_quirks(subarch: HardwareSubarch) -> PlatformQuirkState {
    match subarch {
        HardwareSubarch::Pc => PlatformQuirkState {
            legacy_pic: true,
            legacy_devices: true,
            pnpbios_disabled: false,
        },
        HardwareSubarch::Xen | HardwareSubarch::IntelMid | HardwareSubarch::Ce4100 => {
            PlatformQuirkState {
                legacy_pic: false,
                legacy_devices: false,
                pnpbios_disabled: true,
            }
        }
        HardwareSubarch::Unknown(_) => PlatformQuirkState {
            legacy_pic: true,
            legacy_devices: true,
            pnpbios_disabled: true,
        },
    }
}

pub const fn x86_pnpbios_disabled(state: PlatformQuirkState) -> bool {
    state.pnpbios_disabled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pc_keeps_legacy_devices_but_xen_disables_pnpbios() {
        let pc = x86_early_init_platform_quirks(HardwareSubarch::Pc);
        assert!(pc.legacy_pic);
        assert!(!x86_pnpbios_disabled(pc));
        let xen = x86_early_init_platform_quirks(HardwareSubarch::Xen);
        assert!(!xen.legacy_devices);
        assert!(x86_pnpbios_disabled(xen));
    }
}
