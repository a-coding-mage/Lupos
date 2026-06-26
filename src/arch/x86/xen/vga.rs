//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/xen/vga.c
//! test-origin: linux:vendor/linux/arch/x86/xen/vga.c
//! Xen dom0 VGA console information import.

pub const VIDEO_TYPE_VLFB: u8 = 0x23;
pub const VIDEO_TYPE_EFI: u8 = 0x70;
pub const VIDEO_CAPABILITY_64BIT_BASE: u16 = 1 << 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScreenInfo {
    pub orig_video_mode: u8,
    pub orig_video_is_vga: u8,
    pub orig_video_lines: u16,
    pub orig_video_cols: u16,
    pub orig_video_ega_bx: u16,
    pub orig_video_points: u16,
    pub orig_x: u16,
    pub orig_y: u16,
    pub lfb_width: u16,
    pub lfb_height: u16,
    pub lfb_depth: u16,
    pub lfb_base: u32,
    pub lfb_size: u32,
    pub lfb_linelength: u16,
    pub ext_lfb_base: u32,
    pub capabilities: u16,
    pub vesa_attributes: u16,
}

impl ScreenInfo {
    pub const fn xen_default() -> Self {
        Self {
            orig_video_mode: 3,
            orig_video_is_vga: 1,
            orig_video_lines: 25,
            orig_video_cols: 80,
            orig_video_ega_bx: 3,
            orig_video_points: 16,
            orig_x: 0,
            orig_y: 24,
            lfb_width: 0,
            lfb_height: 0,
            lfb_depth: 0,
            lfb_base: 0,
            lfb_size: 0,
            lfb_linelength: 0,
            ext_lfb_base: 0,
            capabilities: 0,
            vesa_attributes: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum XenVgaInfo {
    TextMode3 {
        rows: u16,
        columns: u16,
        cursor_x: u16,
        cursor_y: u16,
        font_height: u16,
        complete: bool,
    },
    LinearFramebuffer {
        efi: bool,
        width: u16,
        height: u16,
        depth: u16,
        base: u32,
        size: u32,
        line_length: u16,
        ext_base: u32,
        mode_attrs: u16,
        has_ext_base: bool,
        has_mode_attrs: bool,
        complete: bool,
    },
    Unknown,
}

pub fn xen_init_vga(info: XenVgaInfo) -> ScreenInfo {
    let mut screen = ScreenInfo::xen_default();
    match info {
        XenVgaInfo::TextMode3 {
            rows,
            columns,
            cursor_x,
            cursor_y,
            font_height,
            complete: true,
        } => {
            screen.orig_video_lines = rows;
            screen.orig_video_cols = columns;
            screen.orig_x = cursor_x;
            screen.orig_y = cursor_y;
            screen.orig_video_points = font_height;
        }
        XenVgaInfo::LinearFramebuffer {
            efi,
            width,
            height,
            depth,
            base,
            size,
            line_length,
            ext_base,
            mode_attrs,
            has_ext_base,
            has_mode_attrs,
            complete: true,
        } => {
            screen.orig_video_is_vga = if efi { VIDEO_TYPE_EFI } else { VIDEO_TYPE_VLFB };
            screen.lfb_width = width;
            screen.lfb_height = height;
            screen.lfb_depth = depth;
            screen.lfb_base = base;
            screen.lfb_size = size;
            screen.lfb_linelength = line_length;
            if has_ext_base && ext_base != 0 {
                screen.ext_lfb_base = ext_base;
                screen.capabilities |= VIDEO_CAPABILITY_64BIT_BASE;
            }
            if !efi && has_mode_attrs {
                screen.vesa_attributes = mode_attrs;
            }
        }
        _ => {}
    }
    screen
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xen_vga_import_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/xen/vga.c"
        ));
        assert!(source.contains("void __init xen_init_vga"));
        assert!(source.contains("orig_video_mode = 3"));
        assert!(source.contains("orig_video_lines = 25"));
        assert!(source.contains("orig_video_cols = 80"));
        assert!(source.contains("case XEN_VGATYPE_TEXT_MODE_3:"));
        assert!(source.contains("case XEN_VGATYPE_EFI_LFB:"));
        assert!(source.contains("case XEN_VGATYPE_VESA_LFB:"));
        assert!(source.contains("VIDEO_CAPABILITY_64BIT_BASE"));
        assert!(source.contains("vesa_attributes = info->u.vesa_lfb.mode_attrs"));

        let text = xen_init_vga(XenVgaInfo::TextMode3 {
            rows: 30,
            columns: 100,
            cursor_x: 5,
            cursor_y: 6,
            font_height: 14,
            complete: true,
        });
        assert_eq!(text.orig_video_lines, 30);
        assert_eq!(text.orig_video_cols, 100);
        assert_eq!(text.orig_x, 5);

        let lfb = xen_init_vga(XenVgaInfo::LinearFramebuffer {
            efi: false,
            width: 1024,
            height: 768,
            depth: 32,
            base: 0xe000_0000,
            size: 4096,
            line_length: 4096,
            ext_base: 1,
            mode_attrs: 0x19f,
            has_ext_base: true,
            has_mode_attrs: true,
            complete: true,
        });
        assert_eq!(lfb.orig_video_is_vga, VIDEO_TYPE_VLFB);
        assert_eq!(lfb.lfb_width, 1024);
        assert_eq!(lfb.ext_lfb_base, 1);
        assert_eq!(lfb.capabilities, VIDEO_CAPABILITY_64BIT_BASE);
        assert_eq!(lfb.vesa_attributes, 0x19f);
    }
}
