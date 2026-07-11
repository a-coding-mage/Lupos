//! linux-parity: complete
//! linux-source: vendor/linux/lib/fonts/font_8x16.c
//! test-origin: linux:vendor/linux/lib/fonts/font_8x16.c
//! Linux's built-in 8x16 VGA bitmap font.

/// Width of each glyph in pixels.
pub const GLYPH_WIDTH: usize = 8;

/// Height of each glyph in pixels (scanlines).
pub const GLYPH_HEIGHT: usize = 16;

/// Linux `font_vga_8x16.charcount`.
pub const GLYPH_COUNT: usize = 256;

/// Linux `VGA8x16_IDX` from `lib/fonts/font.h`.
pub const VGA8X16_IDX: i32 = 1;

pub const FONT_DATA_LEN: usize = GLYPH_COUNT * GLYPH_HEIGHT;

/// Exact `fontdata_8x16.data` bytes extracted from the vendored Linux source
/// by `build.rs`.
pub static FONT_DATA: &[u8; FONT_DATA_LEN] = include_bytes!(env!("LUPOS_FONT_8X16_BIN"));

/// Rust representation of the Linux `struct font_desc` values relevant to a
/// built-in immutable font.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FontDesc {
    pub idx: i32,
    pub name: &'static str,
    pub width: u32,
    pub height: u32,
    pub charcount: u32,
    pub data: &'static [u8; FONT_DATA_LEN],
    pub pref: i32,
}

/// `font_vga_8x16` from Linux `lib/fonts/font_8x16.c`.
pub static FONT_VGA_8X16: FontDesc = FontDesc {
    idx: VGA8X16_IDX,
    name: "VGA8x16",
    width: GLYPH_WIDTH as u32,
    height: GLYPH_HEIGHT as u32,
    charcount: GLYPH_COUNT as u32,
    data: FONT_DATA,
    pref: 0,
};

/// Return the exact 16 scanline bytes for any Linux console character.
///
/// Linux's table contains all 256 byte-valued glyphs; control and high-bit
/// characters therefore retain their own glyph instead of being rewritten to
/// `?`.
pub fn glyph(ch: u8) -> &'static [u8; GLYPH_HEIGHT] {
    let offset = ch as usize * GLYPH_HEIGHT;
    (&FONT_DATA[offset..offset + GLYPH_HEIGHT])
        .try_into()
        .expect("u8 glyph index is bounded by the 256-glyph Linux table")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_descriptor_matches_linux_font_vga_8x16() {
        assert_eq!(FONT_DATA.len(), 4096);
        assert_eq!(FONT_VGA_8X16.idx, 1);
        assert_eq!(FONT_VGA_8X16.name, "VGA8x16");
        assert_eq!(FONT_VGA_8X16.width, 8);
        assert_eq!(FONT_VGA_8X16.height, 16);
        assert_eq!(FONT_VGA_8X16.charcount, 256);
        assert_eq!(FONT_VGA_8X16.pref, 0);
    }

    #[test]
    fn representative_glyphs_match_linux_source_bytes() {
        // These cover a control glyph, printable ASCII, and a high-bit glyph.
        assert_eq!(
            glyph(0x01),
            &[
                0x00, 0x00, 0x7e, 0x81, 0xa5, 0x81, 0x81, 0xbd, 0x99, 0x81, 0x81, 0x7e, 0x00, 0x00,
                0x00, 0x00,
            ]
        );
        assert_eq!(
            glyph(b'A'),
            &[
                0x00, 0x00, 0x10, 0x38, 0x6c, 0xc6, 0xc6, 0xfe, 0xc6, 0xc6, 0xc6, 0xc6, 0x00, 0x00,
                0x00, 0x00,
            ]
        );
        assert_ne!(glyph(0x80), glyph(b'?'));
    }

    #[test]
    fn every_byte_value_has_a_distinct_table_slot() {
        assert_eq!(glyph(0x00).as_ptr(), FONT_DATA.as_ptr());
        assert_eq!(
            glyph(0xff).as_ptr(),
            FONT_DATA[255 * GLYPH_HEIGHT..].as_ptr()
        );
    }

    #[test]
    fn source_contract_keeps_full_linux_table_and_descriptor() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/fonts/font_8x16.c"
        ));
        assert!(source.contains("#define FONTDATAMAX 4096"));
        assert!(source.contains("static const struct font_data fontdata_8x16"));
        assert!(source.contains("const struct font_desc font_vga_8x16"));
        assert!(source.contains(".charcount = 256"));
        assert!(source.contains(".data\t= fontdata_8x16.data"));
    }
}
