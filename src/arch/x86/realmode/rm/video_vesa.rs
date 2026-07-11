//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/realmode/rm/video-vesa.c
//! test-origin: linux:vendor/linux/arch/x86/realmode/rm/video-vesa.c
//! Real-mode `_WAKEUP` build of Linux `arch/x86/boot/video-vesa.c`.
//!
//! The graphics-mode boot-parameter writer and all EDID/DAC/PM-info routines
//! are absent from this build. Only probing and mode setting are exposed, and
//! graphics mode setting always uses the `_WAKEUP` no-op storage path.

use crate::arch::x86::boot::biosregs::BiosCaller;
use crate::arch::x86::boot::vesa::{FarPtr, VesaGeneralInfo, VesaModeInfo};
use crate::arch::x86::boot::video::{ModeInfo, VideoState};
use crate::arch::x86::boot::video_vesa as boot_vesa;

pub use boot_vesa::{VESA_CARD_NAME, VESA_XMODE_FIRST, VESA_XMODE_N, VesaMem};

/// Build switch that remains relevant in the `_WAKEUP` compile. EDID support
/// and the wakeup selector are intentionally not runtime options here.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct VesaConfig {
    pub vesa_support: bool,
}

impl Default for VesaConfig {
    fn default() -> Self {
        Self { vesa_support: true }
    }
}

fn boot_config(cfg: &VesaConfig) -> boot_vesa::VesaConfig {
    boot_vesa::VesaConfig {
        vesa_support: cfg.vesa_support,
        firmware_edid: false,
        wakeup: true,
    }
}

pub fn vesa_probe<B, M, Q>(
    bios: &B,
    mem: &mut M,
    vginfo: &mut VesaGeneralInfo,
    vminfo: &mut VesaModeInfo,
    cfg: &VesaConfig,
    heap_bytes: usize,
    query_mode: Q,
) -> alloc::vec::Vec<ModeInfo>
where
    B: BiosCaller,
    M: VesaMem,
    Q: FnMut(u16, FarPtr, &mut VesaModeInfo) -> u16,
{
    boot_vesa::vesa_probe(
        bios,
        mem,
        vginfo,
        vminfo,
        &boot_config(cfg),
        heap_bytes,
        query_mode,
    )
}

pub fn vesa_set_mode<B, Q, G>(
    bios: &B,
    vminfo: &mut VesaModeInfo,
    vminfo_destination: FarPtr,
    st: &mut VideoState,
    cfg: &VesaConfig,
    mi: &ModeInfo,
    query_mode: Q,
    store_graphics: G,
) -> i32
where
    B: BiosCaller,
    Q: FnMut(u16, FarPtr, &mut VesaModeInfo) -> u16,
    G: FnMut(&mut VideoState, &VesaModeInfo),
{
    boot_vesa::vesa_set_mode(
        bios,
        vminfo,
        vminfo_destination,
        st,
        &boot_config(cfg),
        mi,
        query_mode,
        store_graphics,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::x86::boot::biosregs::BiosRegs;
    use crate::arch::x86::boot::video::VIDEO_FIRST_VESA;

    struct SetModeBios;

    impl BiosCaller for SetModeBios {
        fn intcall(&self, _int_no: u8, ireg: &BiosRegs, oreg: Option<&mut BiosRegs>) {
            if let Some(out) = oreg {
                *out = BiosRegs::default();
                if ireg.ax() == 0x4f02 {
                    out.set_ax(0x004f);
                }
            }
        }
    }

    #[test]
    fn wrapper_includes_boot_video_vesa_c() {
        assert_eq!(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/vendor/linux/arch/x86/realmode/rm/video-vesa.c"
            ))
            .trim(),
            "#include \"../../boot/video-vesa.c\""
        );

        assert_eq!(VESA_XMODE_FIRST, VIDEO_FIRST_VESA);
    }

    #[test]
    fn wakeup_graphics_mode_never_writes_boot_screen_info() {
        let mut vminfo = VesaModeInfo::default();
        let mut state = VideoState::default();
        state.screen_info.orig_video_isvga = 0x7a;
        let mode = ModeInfo {
            mode: VIDEO_FIRST_VESA + 0x117,
            x: 1024,
            y: 768,
            depth: 32,
        };
        let mut stored = false;

        let result = vesa_set_mode(
            &SetModeBios,
            &mut vminfo,
            FarPtr {
                off: 0x2000,
                seg: 0x9000,
            },
            &mut state,
            &VesaConfig::default(),
            &mode,
            |_mode, destination, info| {
                let destination = (destination.seg, destination.off);
                assert_eq!(destination, (0x9000, 0x2000));
                // mode_attr = 0x0099 (supported color graphics + LFB).
                let bytes = info as *mut VesaModeInfo as *mut u8;
                // SAFETY: VesaModeInfo is a writable 256-byte packed object;
                // offsets 0 and 1 are its little-endian mode_attr field.
                unsafe {
                    bytes.write(0x99);
                    bytes.add(1).write(0x00);
                }
                0x004f
            },
            |_state, _info| stored = true,
        );

        assert_eq!(result, 0);
        assert_eq!(state.graphic_mode, 1);
        assert!(
            !stored,
            "_WAKEUP compiles graphics parameter storage to a no-op"
        );
        assert_eq!(state.screen_info.orig_video_isvga, 0x7a);
    }
}
