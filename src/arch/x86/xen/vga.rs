//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/xen/vga.c
//! test-origin: linux:vendor/linux/arch/x86/xen/vga.c
//! Xen dom0 VGA console information import.

use core::mem::{offset_of, size_of};

pub const XEN_VGATYPE_TEXT_MODE_3: u8 = 0x03;
pub const XEN_VGATYPE_VESA_LFB: u8 = 0x23;
pub const XEN_VGATYPE_EFI_LFB: u8 = 0x70;

pub const VIDEO_TYPE_VLFB: u8 = 0x23;
pub const VIDEO_TYPE_EFI: u8 = 0x70;
pub const VIDEO_CAPABILITY_64BIT_BASE: u32 = 1 << 1;

/// `struct screen_info` from `include/uapi/linux/screen_info.h`.
///
/// `xen_init_vga()` writes only a subset, but keeping the complete packed ABI
/// prevents width/offset drift at the boot-parameter boundary.
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ScreenInfo {
    pub orig_x: u8,
    pub orig_y: u8,
    pub ext_mem_k: u16,
    pub orig_video_page: u16,
    pub orig_video_mode: u8,
    pub orig_video_cols: u8,
    pub flags: u8,
    pub unused2: u8,
    pub orig_video_ega_bx: u16,
    pub unused3: u16,
    pub orig_video_lines: u8,
    pub orig_video_is_vga: u8,
    pub orig_video_points: u16,
    pub lfb_width: u16,
    pub lfb_height: u16,
    pub lfb_depth: u16,
    pub lfb_base: u32,
    pub lfb_size: u32,
    pub cl_magic: u16,
    pub cl_offset: u16,
    pub lfb_linelength: u16,
    pub red_size: u8,
    pub red_pos: u8,
    pub green_size: u8,
    pub green_pos: u8,
    pub blue_size: u8,
    pub blue_pos: u8,
    pub rsvd_size: u8,
    pub rsvd_pos: u8,
    pub vesapm_seg: u16,
    pub vesapm_off: u16,
    pub pages: u16,
    pub vesa_attributes: u16,
    pub capabilities: u32,
    pub ext_lfb_base: u32,
    pub reserved: [u8; 2],
}

/// `dom0_vga_console_info.u.text_mode_3` from Xen's public interface.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct XenTextMode3 {
    pub font_height: u16,
    pub cursor_x: u16,
    pub cursor_y: u16,
    pub rows: u16,
    pub columns: u16,
}

/// `dom0_vga_console_info.u.vesa_lfb` from Xen's public interface.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct XenVesaLfb {
    pub width: u16,
    pub height: u16,
    pub bytes_per_line: u16,
    pub bits_per_pixel: u16,
    pub lfb_base: u32,
    pub lfb_size: u32,
    pub red_pos: u8,
    pub red_size: u8,
    pub green_pos: u8,
    pub green_size: u8,
    pub blue_pos: u8,
    pub blue_size: u8,
    pub rsvd_pos: u8,
    pub rsvd_size: u8,
    pub gbl_caps: u32,
    pub mode_attrs: u16,
    pub pad: u16,
    pub ext_lfb_base: u32,
}

/// Typed view of the active member of `struct dom0_vga_console_info`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum XenVgaInfo {
    TextMode3(XenTextMode3),
    VesaLfb(XenVesaLfb),
    EfiLfb(XenVesaLfb),
    Unknown(u8),
}

// `video_type` is followed by three bytes of C alignment padding before the
// four-byte-aligned union in Xen's public ABI.
const DOM0_VGA_UNION_OFFSET: usize = 4;
const TEXT_MODE_3_END: usize = DOM0_VGA_UNION_OFFSET + size_of::<XenTextMode3>();
const VESA_GBL_CAPS_OFFSET: usize = DOM0_VGA_UNION_OFFSET + offset_of!(XenVesaLfb, gbl_caps);
const VESA_MODE_ATTRS_END: usize =
    DOM0_VGA_UNION_OFFSET + offset_of!(XenVesaLfb, mode_attrs) + size_of::<u16>();
const VESA_EXT_LFB_BASE_END: usize =
    DOM0_VGA_UNION_OFFSET + offset_of!(XenVesaLfb, ext_lfb_base) + size_of::<u32>();

/// Port of `xen_init_vga()` from `arch/x86/xen/vga.c`.
///
/// `size` is the byte count supplied by Xen. Linux deliberately uses three
/// different thresholds: the complete text member, the offset (not the end)
/// of `gbl_caps` for the base LFB fields, and full-field bounds for optional
/// `mode_attrs` and `ext_lfb_base`.
pub fn xen_init_vga(info: XenVgaInfo, size: usize, screen_info: &mut ScreenInfo) {
    // This default block is copied verbatim in intent from vgacon:startup.
    screen_info.orig_video_mode = 3;
    screen_info.orig_video_is_vga = 1;
    screen_info.orig_video_lines = 25;
    screen_info.orig_video_cols = 80;
    screen_info.orig_video_ega_bx = 3;
    screen_info.orig_video_points = 16;
    screen_info.orig_y = screen_info.orig_video_lines - 1;

    match info {
        XenVgaInfo::TextMode3(text) => {
            if size < TEXT_MODE_3_END {
                return;
            }
            // Linux assigns u16 Xen fields into the packed u8 screen fields;
            // the casts preserve C's low-byte truncation exactly.
            screen_info.orig_video_lines = text.rows as u8;
            screen_info.orig_video_cols = text.columns as u8;
            screen_info.orig_x = text.cursor_x as u8;
            screen_info.orig_y = text.cursor_y as u8;
            screen_info.orig_video_points = text.font_height;
        }
        XenVgaInfo::VesaLfb(lfb) | XenVgaInfo::EfiLfb(lfb) => {
            if size < VESA_GBL_CAPS_OFFSET {
                return;
            }

            screen_info.orig_video_is_vga = VIDEO_TYPE_VLFB;
            screen_info.lfb_width = lfb.width;
            screen_info.lfb_height = lfb.height;
            screen_info.lfb_depth = lfb.bits_per_pixel;
            screen_info.lfb_base = lfb.lfb_base;
            screen_info.lfb_size = lfb.lfb_size;
            screen_info.lfb_linelength = lfb.bytes_per_line;
            screen_info.red_size = lfb.red_size;
            screen_info.red_pos = lfb.red_pos;
            screen_info.green_size = lfb.green_size;
            screen_info.green_pos = lfb.green_pos;
            screen_info.blue_size = lfb.blue_size;
            screen_info.blue_pos = lfb.blue_pos;
            screen_info.rsvd_size = lfb.rsvd_size;
            screen_info.rsvd_pos = lfb.rsvd_pos;

            if size >= VESA_EXT_LFB_BASE_END && lfb.ext_lfb_base != 0 {
                screen_info.ext_lfb_base = lfb.ext_lfb_base;
                screen_info.capabilities |= VIDEO_CAPABILITY_64BIT_BASE;
            }

            if matches!(info, XenVgaInfo::EfiLfb(_)) {
                screen_info.orig_video_is_vga = VIDEO_TYPE_EFI;
                return;
            }

            if size >= VESA_MODE_ATTRS_END {
                screen_info.vesa_attributes = lfb.mode_attrs;
            }
        }
        XenVgaInfo::Unknown(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_lfb() -> XenVesaLfb {
        XenVesaLfb {
            width: 1024,
            height: 768,
            bytes_per_line: 4096,
            bits_per_pixel: 32,
            lfb_base: 0xe000_0000,
            lfb_size: 4096,
            red_pos: 16,
            red_size: 8,
            green_pos: 8,
            green_size: 8,
            blue_pos: 0,
            blue_size: 8,
            rsvd_pos: 24,
            rsvd_size: 8,
            gbl_caps: 0,
            mode_attrs: 0x19f,
            pad: 0,
            ext_lfb_base: 1,
        }
    }

    #[test]
    fn xen_vga_abi_layout_matches_linux_headers() {
        assert_eq!(size_of::<ScreenInfo>(), 64);
        assert_eq!(offset_of!(ScreenInfo, capabilities), 0x36);
        assert_eq!(offset_of!(ScreenInfo, ext_lfb_base), 0x3a);
        assert_eq!(size_of::<XenTextMode3>(), 10);
        assert_eq!(offset_of!(XenVesaLfb, gbl_caps), 24);
        assert_eq!(offset_of!(XenVesaLfb, mode_attrs), 28);
        assert_eq!(offset_of!(XenVesaLfb, ext_lfb_base), 32);
        assert_eq!(size_of::<XenVesaLfb>(), 36);
        assert_eq!(VIDEO_CAPABILITY_64BIT_BASE, 1 << 1);
    }

    #[test]
    fn xen_text_import_obeys_exact_size_gate_and_c_field_widths() {
        let text = XenVgaInfo::TextMode3(XenTextMode3 {
            rows: 300,
            columns: 100,
            cursor_x: 261,
            cursor_y: 6,
            font_height: 14,
        });
        let mut short = ScreenInfo::default();
        xen_init_vga(text, TEXT_MODE_3_END - 1, &mut short);
        assert_eq!(short.orig_video_lines, 25);
        assert_eq!(short.orig_video_cols, 80);

        let mut complete = ScreenInfo::default();
        xen_init_vga(text, TEXT_MODE_3_END, &mut complete);
        assert_eq!(complete.orig_video_lines, 300u16 as u8);
        assert_eq!(complete.orig_video_cols, 100);
        assert_eq!(complete.orig_x, 261u16 as u8);
        assert_eq!(complete.orig_y, 6);
        let points = complete.orig_video_points;
        assert_eq!(points, 14);
    }

    #[test]
    fn xen_vesa_import_copies_masks_and_gates_optional_tail_fields() {
        let lfb = sample_lfb();
        let mut screen = ScreenInfo::default();
        xen_init_vga(XenVgaInfo::VesaLfb(lfb), VESA_GBL_CAPS_OFFSET, &mut screen);
        let width = screen.lfb_width;
        assert_eq!(width, 1024);
        assert_eq!(screen.red_size, 8);
        assert_eq!(screen.red_pos, 16);
        assert_eq!(screen.green_pos, 8);
        assert_eq!(screen.blue_pos, 0);
        assert_eq!(screen.rsvd_pos, 24);
        let attrs = screen.vesa_attributes;
        let ext = screen.ext_lfb_base;
        let caps = screen.capabilities;
        assert_eq!(attrs, 0);
        assert_eq!(ext, 0);
        assert_eq!(caps, 0);

        xen_init_vga(XenVgaInfo::VesaLfb(lfb), VESA_EXT_LFB_BASE_END, &mut screen);
        let attrs = screen.vesa_attributes;
        let ext = screen.ext_lfb_base;
        let caps = screen.capabilities;
        assert_eq!(attrs, 0x19f);
        assert_eq!(ext, 1);
        assert_eq!(caps, VIDEO_CAPABILITY_64BIT_BASE);
    }

    #[test]
    fn xen_efi_import_returns_before_vesa_mode_attributes() {
        let mut screen = ScreenInfo::default();
        xen_init_vga(
            XenVgaInfo::EfiLfb(sample_lfb()),
            VESA_EXT_LFB_BASE_END,
            &mut screen,
        );
        assert_eq!(screen.orig_video_is_vga, VIDEO_TYPE_EFI);
        let attrs = screen.vesa_attributes;
        assert_eq!(attrs, 0);
    }

    #[test]
    fn xen_vga_source_contract_names_all_size_gates() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/xen/vga.c"
        ));
        assert!(source.contains("sizeof(info->u.text_mode_3)"));
        assert!(source.contains("u.vesa_lfb.gbl_caps"));
        assert!(source.contains("u.vesa_lfb.mode_attrs"));
        assert!(source.contains("u.vesa_lfb.ext_lfb_base"));
        assert!(source.contains("VIDEO_CAPABILITY_64BIT_BASE"));
    }
}
