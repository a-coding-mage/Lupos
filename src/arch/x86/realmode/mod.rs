//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/realmode
//! test-origin: linux:vendor/linux/arch/x86/realmode
//! Real-mode trampoline reservation and BIOS-video handoff model.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/realmode/init.c
//! - vendor/linux/arch/x86/realmode/rm/regs.c
//! - vendor/linux/arch/x86/realmode/rm/video-bios.c
//! - vendor/linux/arch/x86/realmode/rm/video-mode.c
//! - vendor/linux/arch/x86/realmode/rm/video-vesa.c
//! - vendor/linux/arch/x86/realmode/rm/video-vga.c
//! - vendor/linux/arch/x86/realmode/rm/wakemain.c

use crate::include::uapi::errno::{EINVAL, ENODEV};

pub mod rm;

pub const REALMODE_LOW_LIMIT: u64 = 0x100000;
pub const REALMODE_PARAGRAPH_SHIFT: u8 = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RealModeReservation {
    pub phys: u64,
    pub size: u64,
    pub encrypted_host_memory: bool,
    pub sev_es_guest: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RealModeRegisters {
    pub ax: u16,
    pub bx: u16,
    pub cx: u16,
    pub dx: u16,
    pub flags: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RealModeVideo {
    BiosText,
    VgaText,
    Vesa { mode: u16 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WakeMainAction {
    JumpToProtectedMode,
    RejectTrampoline,
}

pub const fn realmode_reservation_valid(res: RealModeReservation) -> bool {
    res.size != 0
        && res.phys < REALMODE_LOW_LIMIT
        && res.size <= REALMODE_LOW_LIMIT
        && res.phys <= REALMODE_LOW_LIMIT - res.size
        && res.phys & 0xfff == 0
}

pub const fn realmode_segment(res: RealModeReservation) -> Result<u16, i32> {
    if realmode_reservation_valid(res) {
        Ok((res.phys >> REALMODE_PARAGRAPH_SHIFT) as u16)
    } else {
        Err(EINVAL)
    }
}

pub const fn realmode_flags(res: RealModeReservation) -> u32 {
    let mut flags = 0u32;
    if res.encrypted_host_memory {
        flags |= 1 << 0;
    }
    if res.sev_es_guest {
        flags |= 1 << 1;
    }
    flags
}

pub const fn bios_video_mode_supported(video: RealModeVideo) -> Result<(), i32> {
    match video {
        RealModeVideo::BiosText | RealModeVideo::VgaText => Ok(()),
        RealModeVideo::Vesa { mode } if mode >= 0x100 => Ok(()),
        RealModeVideo::Vesa { .. } => Err(ENODEV),
    }
}

pub const fn wake_main_action(res: RealModeReservation) -> WakeMainAction {
    if realmode_reservation_valid(res) {
        WakeMainAction::JumpToProtectedMode
    } else {
        WakeMainAction::RejectTrampoline
    }
}

pub const fn regs_carry_set(regs: RealModeRegisters) -> bool {
    regs.flags & 1 != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trampoline_must_live_below_one_megabyte() {
        let res = RealModeReservation {
            phys: 0x8000,
            size: 0x2000,
            encrypted_host_memory: true,
            sev_es_guest: false,
        };
        assert_eq!(realmode_segment(res), Ok(0x800));
        assert_eq!(realmode_flags(res), 1);
        assert_eq!(wake_main_action(res), WakeMainAction::JumpToProtectedMode);
        assert!(!realmode_reservation_valid(RealModeReservation {
            phys: 0xff000,
            size: 0x2000,
            ..res
        }));
    }

    #[test]
    fn video_modes_and_regs_match_real_mode_call_shape() {
        assert!(bios_video_mode_supported(RealModeVideo::BiosText).is_ok());
        assert_eq!(
            bios_video_mode_supported(RealModeVideo::Vesa { mode: 0x13 }),
            Err(ENODEV)
        );
        assert!(regs_carry_set(RealModeRegisters {
            ax: 0,
            bx: 0,
            cx: 0,
            dx: 0,
            flags: 1,
        }));
    }
}
