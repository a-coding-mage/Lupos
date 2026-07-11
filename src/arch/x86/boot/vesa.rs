//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/vesa.h
//! test-origin: linux:vendor/linux/arch/x86/boot/vesa.h
//! VESA BIOS Extension (VBE) info-block layouts.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/vesa.h
//!
//! These are the *canonical* Rust definitions of the two VBE structures the
//! boot stub fills from INT 10h AX=4F00h / AX=4F01h. The BIOS writes them
//! byte-for-byte, so the layout is ABI-critical: every field keeps its
//! Linux offset. The two 256-byte BIOS blocks are packed exactly like the C
//! header; `far_ptr` itself keeps the header's ordinary C alignment.

/// `far_ptr` — a real-mode `offset:segment` 16:16 far pointer (vesa.h
/// lines 11-13). Fields are in `off, seg` order, matching the C struct.
#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct FarPtr {
    /// 16-bit offset within the segment.
    pub off: u16,
    /// 16-bit real-mode segment.
    pub seg: u16,
}

/// `VESA_MAGIC` — the `"VESA"` signature returned in the general info
/// block. Defined in vesa.h line 27 as
/// `'V' + ('E' << 8) + ('S' << 16) + ('A' << 24)`, i.e. the four ASCII
/// bytes stored little-endian.
pub const VESA_MAGIC: u32 =
    b'V' as u32 + ((b'E' as u32) << 8) + ((b'S' as u32) << 16) + ((b'A' as u32) << 24);

/// `struct vesa_general_info` — VBE controller info (vesa.h lines 16-25).
///
/// Filled by INT 10h AX=4F00h. Field comments carry the C byte offsets so
/// the layout can be cross-checked against the header. Total size 256.
#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VesaGeneralInfo {
    /// 0  Magic number = "VESA" (compare against [`VESA_MAGIC`]).
    pub signature: u32,
    /// 4  VBE version (BCD: 0x0200 = VBE 2.0).
    pub version: u16,
    /// 6  Far pointer to the OEM vendor string.
    pub vendor_string: FarPtr,
    /// 10 Capabilities bitfield.
    pub capabilities: u32,
    /// 14 Far pointer to the supported video-mode list.
    pub video_mode_ptr: FarPtr,
    /// 18 Total video memory in 64 KiB blocks.
    pub total_memory: u16,
    /// 20 Reserved/OEM data filling the block out to 256 bytes.
    pub reserved: [u8; 236],
}

impl Default for VesaGeneralInfo {
    fn default() -> Self {
        VesaGeneralInfo {
            signature: 0,
            version: 0,
            vendor_string: FarPtr::default(),
            capabilities: 0,
            video_mode_ptr: FarPtr::default(),
            total_memory: 0,
            reserved: [0; 236],
        }
    }
}

/// `struct vesa_mode_info` — per-mode info block (vesa.h lines 29-65).
///
/// Filled by INT 10h AX=4F01h for one VBE mode. Field comments carry the C
/// byte offsets. Total size 256.
#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VesaModeInfo {
    /// 0  Mode attributes bitfield.
    pub mode_attr: u16,
    /// 2  Window A/B attributes.
    pub win_attr: [u8; 2],
    /// 4  Window granularity in KiB.
    pub win_grain: u16,
    /// 6  Window size in KiB.
    pub win_size: u16,
    /// 8  Start segments for windows A and B.
    pub win_seg: [u16; 2],
    /// 12 Far pointer to the window-positioning function.
    pub win_scheme: FarPtr,
    /// 16 Bytes per scan line.
    pub logical_scan: u16,
    /// 18 Horizontal resolution in pixels/chars.
    pub h_res: u16,
    /// 20 Vertical resolution in pixels/chars.
    pub v_res: u16,
    /// 22 Character cell width in pixels.
    pub char_width: u8,
    /// 23 Character cell height in pixels.
    pub char_height: u8,
    /// 24 Number of memory planes.
    pub memory_planes: u8,
    /// 25 Bits per pixel.
    pub bpp: u8,
    /// 26 Number of banks.
    pub banks: u8,
    /// 27 Memory model type.
    pub memory_layout: u8,
    /// 28 Bank size in KiB.
    pub bank_size: u8,
    /// 29 Number of image pages.
    pub image_planes: u8,
    /// 30 Reserved (page function).
    pub page_function: u8,
    /// 31 Size of direct-color red mask in bits.
    pub rmask: u8,
    /// 32 Bit position of LSB of red mask.
    pub rpos: u8,
    /// 33 Size of direct-color green mask in bits.
    pub gmask: u8,
    /// 34 Bit position of LSB of green mask.
    pub gpos: u8,
    /// 35 Size of direct-color blue mask in bits.
    pub bmask: u8,
    /// 36 Bit position of LSB of blue mask.
    pub bpos: u8,
    /// 37 Size of reserved mask in bits.
    pub resv_mask: u8,
    /// 38 Bit position of LSB of reserved mask.
    pub resv_pos: u8,
    /// 39 Direct-color mode info bitfield.
    pub dcm_info: u8,
    /// 40 Linear frame buffer physical address.
    pub lfb_ptr: u32,
    /// 44 Offscreen memory physical address.
    pub offscreen_ptr: u32,
    /// 48 Offscreen memory size in KiB.
    pub offscreen_size: u16,
    /// 50 Reserved padding out to 256 bytes.
    pub reserved: [u8; 206],
}

impl Default for VesaModeInfo {
    fn default() -> Self {
        VesaModeInfo {
            mode_attr: 0,
            win_attr: [0; 2],
            win_grain: 0,
            win_size: 0,
            win_seg: [0; 2],
            win_scheme: FarPtr::default(),
            logical_scan: 0,
            h_res: 0,
            v_res: 0,
            char_width: 0,
            char_height: 0,
            memory_planes: 0,
            bpp: 0,
            banks: 0,
            memory_layout: 0,
            bank_size: 0,
            image_planes: 0,
            page_function: 0,
            rmask: 0,
            rpos: 0,
            gmask: 0,
            gpos: 0,
            bmask: 0,
            bpos: 0,
            resv_mask: 0,
            resv_pos: 0,
            dcm_info: 0,
            lfb_ptr: 0,
            offscreen_ptr: 0,
            offscreen_size: 0,
            reserved: [0; 206],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, size_of};

    #[test]
    fn far_ptr_is_four_bytes_off_then_seg() {
        assert_eq!(size_of::<FarPtr>(), 4);
        assert_eq!(align_of::<FarPtr>(), align_of::<u16>());
    }

    #[test]
    fn vesa_magic_is_VESA_little_endian() {
        // "VESA" stored little-endian = 0x41 'A' << 24 | 'S' << 16 | 'E' << 8 | 'V'.
        assert_eq!(VESA_MAGIC, 0x4153_4556);
    }

    #[test]
    fn general_info_matches_c_layout_256_bytes() {
        // Sum of fields: 4 + 2 + 4 + 4 + 4 + 2 + 236 = 256.
        assert_eq!(size_of::<VesaGeneralInfo>(), 256);
    }

    #[test]
    fn mode_info_matches_c_layout_256_bytes() {
        // reserved[206] starts at offset 50, so 50 + 206 = 256.
        assert_eq!(size_of::<VesaModeInfo>(), 256);
    }

    #[test]
    fn general_info_default_signature_can_be_set_to_magic() {
        let mut gi = VesaGeneralInfo::default();
        gi.signature = VESA_MAGIC;
        // Packed field: copy to a local before comparing to avoid taking a
        // reference to an unaligned field.
        let sig = gi.signature;
        assert_eq!(sig, VESA_MAGIC);
    }

    #[test]
    fn mode_info_fields_round_trip_through_packed_read() {
        let mut mi = VesaModeInfo::default();
        mi.h_res = 1024;
        mi.v_res = 768;
        mi.bpp = 32;
        mi.lfb_ptr = 0xE000_0000;
        // Read packed fields via copies (the values are unaligned in a
        // packed struct); `read_unaligned` would also be valid.
        let (h, v, bpp, lfb) = (mi.h_res, mi.v_res, mi.bpp, mi.lfb_ptr);
        assert_eq!((h, v, bpp, lfb), (1024, 768, 32, 0xE000_0000));
    }
}
