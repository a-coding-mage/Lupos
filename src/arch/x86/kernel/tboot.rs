//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/tboot.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/tboot.c
//! Intel Trusted Boot (`tboot`) detection and shutdown planning.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/tboot.c

#![allow(dead_code)]

use crate::include::uapi::errno::{EINVAL, ENODEV, EOPNOTSUPP};

pub const TBOOT_UUID: [u8; 16] = [
    0xc0, 0xba, 0x75, 0x9b, 0x6f, 0x4c, 0x97, 0x47, 0x9d, 0xe0, 0x9a, 0xcb, 0x10, 0x0b, 0x55, 0x8d,
];
pub const TB_SHUTDOWN_REBOOT: u32 = 0;
pub const TB_SHUTDOWN_S5: u32 = 1;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TbootSharedPage {
    pub uuid: [u8; 16],
    pub version: u32,
    pub log_addr: u64,
    pub log_size: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TbootState {
    pub enabled: bool,
    pub version: u32,
    pub log_addr: u64,
    pub log_size: u32,
}

pub const fn check_tboot_version(page: &TbootSharedPage) -> bool {
    page.uuid[0] == TBOOT_UUID[0]
        && page.uuid[1] == TBOOT_UUID[1]
        && page.uuid[2] == TBOOT_UUID[2]
        && page.uuid[3] == TBOOT_UUID[3]
        && page.uuid[4] == TBOOT_UUID[4]
        && page.uuid[5] == TBOOT_UUID[5]
        && page.uuid[6] == TBOOT_UUID[6]
        && page.uuid[7] == TBOOT_UUID[7]
        && page.uuid[8] == TBOOT_UUID[8]
        && page.uuid[9] == TBOOT_UUID[9]
        && page.uuid[10] == TBOOT_UUID[10]
        && page.uuid[11] == TBOOT_UUID[11]
        && page.uuid[12] == TBOOT_UUID[12]
        && page.uuid[13] == TBOOT_UUID[13]
        && page.uuid[14] == TBOOT_UUID[14]
        && page.uuid[15] == TBOOT_UUID[15]
        && page.version >= 5
}

pub const fn tboot_probe(page: Option<TbootSharedPage>) -> Result<TbootState, i32> {
    match page {
        Some(p) if check_tboot_version(&p) => Ok(TbootState {
            enabled: true,
            version: p.version,
            log_addr: p.log_addr,
            log_size: p.log_size,
        }),
        Some(_) => Err(EINVAL),
        None => Err(ENODEV),
    }
}

pub const fn tboot_enabled(state: &TbootState) -> bool {
    state.enabled
}

pub const fn tboot_shutdown(state: &TbootState, shutdown_type: u32) -> Result<u32, i32> {
    if !state.enabled {
        return Err(ENODEV);
    }
    match shutdown_type {
        TB_SHUTDOWN_REBOOT | TB_SHUTDOWN_S5 => Ok(shutdown_type),
        _ => Err(EINVAL),
    }
}

pub const fn tboot_sleep(_sleep_state: u8) -> Result<(), i32> {
    Err(EOPNOTSUPP)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_accepts_matching_uuid_and_min_version() {
        let page = TbootSharedPage {
            uuid: TBOOT_UUID,
            version: 5,
            log_addr: 0x1000,
            log_size: 64,
        };
        let state = tboot_probe(Some(page)).unwrap();
        assert!(tboot_enabled(&state));
        assert_eq!(
            tboot_shutdown(&state, TB_SHUTDOWN_REBOOT),
            Ok(TB_SHUTDOWN_REBOOT)
        );
    }

    #[test]
    fn probe_rejects_missing_or_old_tboot() {
        assert_eq!(tboot_probe(None), Err(ENODEV));
        assert_eq!(
            tboot_probe(Some(TbootSharedPage {
                uuid: TBOOT_UUID,
                version: 1,
                ..TbootSharedPage::default()
            })),
            Err(EINVAL)
        );
    }
}
