//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/include/uapi/asm/bootparam.h
//! test-origin: linux:vendor/linux/arch/x86/include/uapi/asm/bootparam.h
//! Minimal Linux `boot_params` (zeropage) helper for Milestone 3.5.
//!
//! The real `struct boot_params` is ~4 KiB. We model it as an opaque
//! 4096-byte blob and provide setters/getters for the fields we actually
//! populate:
//!   - sentinel (0x1ef)
//!   - e820_entries (0x1e8) and e820_table (0x2d0, 128 entries)
//!   - screen_info (subset) at offset 0x00
//! This keeps layout compatibility without re-declaring the full header.

use crate::arch::x86::boot::compressed::efi::EfiInfo;

pub const BOOT_PARAMS_SIZE: usize = 4096;
pub const E820_MAX: usize = 128;

// Offsets inside boot_params (see linux/arch/x86/include/uapi/asm/bootparam.h)
const OFF_E820_ENTRIES: usize = 0x1e8;
const OFF_SENTINEL: usize = 0x1ef;
const OFF_ACPI_RSDP_ADDR: usize = 0x070;
const OFF_HDR_VERSION: usize = 0x206;
const OFF_HDR_VID_MODE: usize = 0x1fa;
const OFF_HDR_LOADFLAGS: usize = 0x211;
const OFF_HDR_HARDWARE_SUBARCH: usize = 0x23c;
const OFF_HDR_HARDWARE_SUBARCH_DATA: usize = 0x240;
const OFF_HDR_RAMDISK_IMAGE: usize = 0x218;
const OFF_HDR_RAMDISK_SIZE: usize = 0x21c;
const OFF_HDR_CMD_LINE_PTR: usize = 0x228;
const OFF_HDR_SETUP_DATA: usize = 0x250;
const OFF_EXT_RAMDISK_IMAGE: usize = 0x0c0;
const OFF_EXT_RAMDISK_SIZE: usize = 0x0c4;
const OFF_EXT_CMD_LINE_PTR: usize = 0x0c8;
const OFF_EFI_INFO: usize = 0x1c0;
const OFF_E820_TABLE: usize = 0x2d0;

// screen_info field offsets (uapi/linux/screen_info.h)
const OFF_SCREEN_ORIG_VIDEO_MODE: usize = 0x06;
const OFF_SCREEN_ORIG_VIDEO_COLS: usize = 0x07;
const OFF_SCREEN_ORIG_VIDEO_LINES: usize = 0x0e;
const OFF_SCREEN_ORIG_VIDEO_ISVGA: usize = 0x0f;
const OFF_SCREEN_LFB_WIDTH: usize = 0x12;
const OFF_SCREEN_LFB_HEIGHT: usize = 0x14;
const OFF_SCREEN_LFB_DEPTH: usize = 0x16;
const OFF_SCREEN_LFB_BASE: usize = 0x18;
const OFF_SCREEN_LFB_SIZE: usize = 0x1c;
const OFF_SCREEN_LFB_LINELENGTH: usize = 0x24;
const OFF_SCREEN_CAPABILITIES: usize = 0x36;
const OFF_SCREEN_EXT_LFB_BASE: usize = 0x3a;

// Video type constants (uapi/linux/screen_info.h)
const VIDEO_TYPE_VGAC: u8 = 0x22;
const VIDEO_TYPE_VLFB: u8 = 0x23;
const VIDEO_CAPABILITY_64BIT_BASE: u32 = 1 << 1;

/// A single E820 entry (20 bytes)
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BootE820Entry {
    pub base_addr: u64,
    pub length: u64,
    pub region_type: u32,
}

#[derive(Debug)]
pub struct BootParams {
    pub data: [u8; BOOT_PARAMS_SIZE],
}

impl BootParams {
    pub const fn new() -> Self {
        Self {
            data: [0u8; BOOT_PARAMS_SIZE],
        }
    }

    pub fn zero(&mut self) {
        self.data = [0u8; BOOT_PARAMS_SIZE];
    }

    pub fn set_sentinel(&mut self) {
        self.data[OFF_SENTINEL] = 0xFF;
    }

    pub fn set_e820_entry(&mut self, idx: usize, entry: BootE820Entry) {
        if idx >= E820_MAX {
            return;
        }
        let offset = OFF_E820_TABLE + idx * 20;
        self.data[offset..offset + 8].copy_from_slice(&entry.base_addr.to_le_bytes());
        self.data[offset + 8..offset + 16].copy_from_slice(&entry.length.to_le_bytes());
        self.data[offset + 16..offset + 20].copy_from_slice(&entry.region_type.to_le_bytes());
    }

    pub fn set_e820_entries(&mut self, count: u8) {
        self.data[OFF_E820_ENTRIES] = count;
    }

    pub fn e820_entries(&self) -> u8 {
        self.data[OFF_E820_ENTRIES]
    }

    pub fn e820_iter(&self) -> impl Iterator<Item = BootE820Entry> + '_ {
        let count = self.e820_entries().min(E820_MAX as u8) as usize;
        (0..count).map(move |i| {
            let offset = OFF_E820_TABLE + i * 20;
            let base = u64::from_le_bytes(self.data[offset..offset + 8].try_into().unwrap());
            let length = u64::from_le_bytes(self.data[offset + 8..offset + 16].try_into().unwrap());
            let region_type =
                u32::from_le_bytes(self.data[offset + 16..offset + 20].try_into().unwrap());
            BootE820Entry {
                base_addr: base,
                length,
                region_type,
            }
        })
    }

    /// Linux `boot_params.alt_mem_k`.
    ///
    /// Ref: vendor/linux/arch/x86/boot/memory.c
    pub fn set_alt_mem_k(&mut self, value: u32) {
        write_u32(&mut self.data, 0x1e0, value);
    }

    pub fn alt_mem_k(&self) -> u32 {
        read_u32(&self.data, 0x1e0)
    }

    /// Linux `boot_params.screen_info.ext_mem_k`.
    ///
    /// Ref: vendor/linux/include/uapi/linux/screen_info.h
    pub fn set_screen_ext_mem_k(&mut self, value: u16) {
        write_u16(&mut self.data, 0x02, value);
    }

    pub fn screen_ext_mem_k(&self) -> u16 {
        read_u16(&self.data, 0x02)
    }

    /// Linux `boot_params.hdr.vid_mode`.
    ///
    /// Ref: vendor/linux/arch/x86/boot/video-mode.c
    pub fn set_video_mode(&mut self, value: u16) {
        write_u16(&mut self.data, OFF_HDR_VID_MODE, value);
    }

    pub fn video_mode(&self) -> u16 {
        read_u16(&self.data, OFF_HDR_VID_MODE)
    }

    /// Linux `boot_params.hdr.version`.
    ///
    /// Ref: vendor/linux/arch/x86/include/uapi/asm/bootparam.h
    pub fn boot_header_version(&self) -> u16 {
        read_u16(&self.data, OFF_HDR_VERSION)
    }

    /// Linux `boot_params.acpi_rsdp_addr`.
    ///
    /// Ref: vendor/linux/arch/x86/include/uapi/asm/bootparam.h
    pub fn acpi_rsdp_addr(&self) -> u64 {
        read_u64(&self.data, OFF_ACPI_RSDP_ADDR)
    }

    pub fn set_acpi_rsdp_addr(&mut self, addr: u64) {
        write_u64(&mut self.data, OFF_ACPI_RSDP_ADDR, addr);
    }

    /// Linux `boot_params.hdr.loadflags`.
    ///
    /// Ref: vendor/linux/arch/x86/include/uapi/asm/bootparam.h
    pub fn loadflags(&self) -> u8 {
        self.data[OFF_HDR_LOADFLAGS]
    }

    pub fn set_loadflags(&mut self, flags: u8) {
        self.data[OFF_HDR_LOADFLAGS] = flags;
    }

    pub fn clear_loadflags(&mut self, mask: u8) {
        self.data[OFF_HDR_LOADFLAGS] &= !mask;
    }

    /// Linux `boot_params.hdr.setup_data`: physical address of the setup_data list.
    ///
    /// Ref: vendor/linux/arch/x86/kernel/ksysfs.c
    pub fn setup_data(&self) -> u64 {
        read_u64(&self.data, OFF_HDR_SETUP_DATA)
    }

    /// Linux `boot_params.hdr.ramdisk_image` plus `ext_ramdisk_image`.
    ///
    /// Ref: vendor/linux/arch/x86/kernel/setup.c
    pub fn ramdisk_image(&self) -> u64 {
        read_u32(&self.data, OFF_HDR_RAMDISK_IMAGE) as u64
            | ((read_u32(&self.data, OFF_EXT_RAMDISK_IMAGE) as u64) << 32)
    }

    /// Linux `boot_params.hdr.ramdisk_size` plus `ext_ramdisk_size`.
    ///
    /// Ref: vendor/linux/arch/x86/kernel/setup.c
    pub fn ramdisk_size(&self) -> u64 {
        read_u32(&self.data, OFF_HDR_RAMDISK_SIZE) as u64
            | ((read_u32(&self.data, OFF_EXT_RAMDISK_SIZE) as u64) << 32)
    }

    /// Linux `boot_params.hdr.cmd_line_ptr` plus `ext_cmd_line_ptr`.
    ///
    /// Ref: vendor/linux/arch/x86/include/uapi/asm/bootparam.h
    pub fn cmd_line_ptr(&self) -> u64 {
        read_u32(&self.data, OFF_HDR_CMD_LINE_PTR) as u64
            | ((read_u32(&self.data, OFF_EXT_CMD_LINE_PTR) as u64) << 32)
    }

    /// Linux `boot_params.efi_info`.
    ///
    /// Ref: vendor/linux/arch/x86/include/uapi/asm/bootparam.h
    pub fn efi_info(&self) -> EfiInfo {
        EfiInfo {
            efi_loader_signature: self.data[OFF_EFI_INFO..OFF_EFI_INFO + 4]
                .try_into()
                .unwrap(),
            efi_systab: read_u32(&self.data, OFF_EFI_INFO + 4),
            efi_memdesc_size: read_u32(&self.data, OFF_EFI_INFO + 8),
            efi_memdesc_version: read_u32(&self.data, OFF_EFI_INFO + 12),
            efi_memmap: read_u32(&self.data, OFF_EFI_INFO + 16),
            efi_memmap_size: read_u32(&self.data, OFF_EFI_INFO + 20),
            efi_systab_hi: read_u32(&self.data, OFF_EFI_INFO + 24),
            efi_memmap_hi: read_u32(&self.data, OFF_EFI_INFO + 28),
        }
    }

    /// Linux `boot_params.hdr.hardware_subarch`.
    ///
    /// Ref: vendor/linux/arch/x86/kernel/platform-quirks.c
    pub fn hardware_subarch(&self) -> u32 {
        read_u32(&self.data, OFF_HDR_HARDWARE_SUBARCH)
    }

    /// Linux `boot_params.hdr.hardware_subarch_data`.
    pub fn hardware_subarch_data(&self) -> u64 {
        read_u64(&self.data, OFF_HDR_HARDWARE_SUBARCH_DATA)
    }

    pub fn set_boot_header_version(&mut self, version: u16) {
        write_u16(&mut self.data, OFF_HDR_VERSION, version);
    }

    pub fn set_setup_data(&mut self, setup_data: u64) {
        write_u64(&mut self.data, OFF_HDR_SETUP_DATA, setup_data);
    }

    pub fn set_ramdisk_image(&mut self, image: u64) {
        write_u32(&mut self.data, OFF_HDR_RAMDISK_IMAGE, image as u32);
        write_u32(&mut self.data, OFF_EXT_RAMDISK_IMAGE, (image >> 32) as u32);
    }

    pub fn set_ramdisk_size(&mut self, size: u64) {
        write_u32(&mut self.data, OFF_HDR_RAMDISK_SIZE, size as u32);
        write_u32(&mut self.data, OFF_EXT_RAMDISK_SIZE, (size >> 32) as u32);
    }

    pub fn set_cmd_line_ptr(&mut self, ptr: u64) {
        write_u32(&mut self.data, OFF_HDR_CMD_LINE_PTR, ptr as u32);
        write_u32(&mut self.data, OFF_EXT_CMD_LINE_PTR, (ptr >> 32) as u32);
    }

    pub fn set_efi_info(&mut self, info: EfiInfo) {
        self.data[OFF_EFI_INFO..OFF_EFI_INFO + 4].copy_from_slice(&info.efi_loader_signature);
        write_u32(&mut self.data, OFF_EFI_INFO + 4, info.efi_systab);
        write_u32(&mut self.data, OFF_EFI_INFO + 8, info.efi_memdesc_size);
        write_u32(&mut self.data, OFF_EFI_INFO + 12, info.efi_memdesc_version);
        write_u32(&mut self.data, OFF_EFI_INFO + 16, info.efi_memmap);
        write_u32(&mut self.data, OFF_EFI_INFO + 20, info.efi_memmap_size);
        write_u32(&mut self.data, OFF_EFI_INFO + 24, info.efi_systab_hi);
        write_u32(&mut self.data, OFF_EFI_INFO + 28, info.efi_memmap_hi);
    }

    pub fn set_hardware_subarch(&mut self, subarch: u32) {
        write_u32(&mut self.data, OFF_HDR_HARDWARE_SUBARCH, subarch);
    }

    pub fn set_hardware_subarch_data(&mut self, data: u64) {
        write_u64(&mut self.data, OFF_HDR_HARDWARE_SUBARCH_DATA, data);
    }

    pub fn set_screen_info_vga_text(&mut self) {
        self.data[OFF_SCREEN_ORIG_VIDEO_ISVGA] = VIDEO_TYPE_VGAC;
        self.data[OFF_SCREEN_ORIG_VIDEO_COLS] = 80;
        self.data[OFF_SCREEN_ORIG_VIDEO_LINES] = 25;
    }

    pub fn set_screen_orig_video_mode(&mut self, mode: u8) {
        self.data[OFF_SCREEN_ORIG_VIDEO_MODE] = mode;
    }

    pub fn screen_orig_video_mode(&self) -> u8 {
        self.data[OFF_SCREEN_ORIG_VIDEO_MODE]
    }

    pub fn set_screen_orig_video_cols(&mut self, cols: u8) {
        self.data[OFF_SCREEN_ORIG_VIDEO_COLS] = cols;
    }

    pub fn screen_orig_video_cols(&self) -> u8 {
        self.data[OFF_SCREEN_ORIG_VIDEO_COLS]
    }

    pub fn set_screen_orig_video_lines(&mut self, lines: u8) {
        self.data[OFF_SCREEN_ORIG_VIDEO_LINES] = lines;
    }

    pub fn screen_orig_video_lines(&self) -> u8 {
        self.data[OFF_SCREEN_ORIG_VIDEO_LINES]
    }

    pub fn set_screen_info_framebuffer(
        &mut self,
        width: u32,
        height: u32,
        bpp: u32,
        pitch: u32,
        fb_addr: u64,
    ) {
        self.data[OFF_SCREEN_ORIG_VIDEO_ISVGA] = VIDEO_TYPE_VLFB;
        write_u16(&mut self.data, OFF_SCREEN_LFB_WIDTH, width as u16);
        write_u16(&mut self.data, OFF_SCREEN_LFB_HEIGHT, height as u16);
        write_u16(&mut self.data, OFF_SCREEN_LFB_DEPTH, bpp as u16);
        write_u32(&mut self.data, OFF_SCREEN_LFB_BASE, fb_addr as u32);
        write_u32(
            &mut self.data,
            OFF_SCREEN_EXT_LFB_BASE,
            (fb_addr >> 32) as u32,
        );
        write_u32(&mut self.data, OFF_SCREEN_LFB_SIZE, 0); // optional
        write_u16(&mut self.data, OFF_SCREEN_LFB_LINELENGTH, pitch as u16);

        if fb_addr >> 32 != 0 {
            write_u32(
                &mut self.data,
                OFF_SCREEN_CAPABILITIES,
                VIDEO_CAPABILITY_64BIT_BASE,
            );
        }
    }

    pub fn framebuffer_info(&self) -> Option<FramebufferInfo> {
        let video = self.data[OFF_SCREEN_ORIG_VIDEO_ISVGA];
        if video != VIDEO_TYPE_VLFB {
            return None;
        }
        let width = read_u16(&self.data, OFF_SCREEN_LFB_WIDTH) as u32;
        let height = read_u16(&self.data, OFF_SCREEN_LFB_HEIGHT) as u32;
        let depth = read_u16(&self.data, OFF_SCREEN_LFB_DEPTH) as u32;
        let pitch = read_u16(&self.data, OFF_SCREEN_LFB_LINELENGTH) as u32;
        let base_low = read_u32(&self.data, OFF_SCREEN_LFB_BASE) as u64;
        let base_high = read_u32(&self.data, OFF_SCREEN_EXT_LFB_BASE) as u64;
        let addr = base_low | (base_high << 32);
        Some(FramebufferInfo {
            addr,
            width,
            height,
            depth,
            pitch,
        })
    }
}

pub struct FramebufferInfo {
    pub addr: u64,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub pitch: u32,
}

fn write_u16(buf: &mut [u8], offset: usize, val: u16) {
    buf[offset..offset + 2].copy_from_slice(&val.to_le_bytes());
}

fn write_u32(buf: &mut [u8], offset: usize, val: u32) {
    buf[offset..offset + 4].copy_from_slice(&val.to_le_bytes());
}

fn write_u64(buf: &mut [u8], offset: usize, val: u64) {
    buf[offset..offset + 8].copy_from_slice(&val.to_le_bytes());
}

fn read_u16(buf: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(buf[offset..offset + 2].try_into().unwrap())
}

fn read_u32(buf: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(buf[offset..offset + 4].try_into().unwrap())
}

fn read_u64(buf: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(buf[offset..offset + 8].try_into().unwrap())
}

#[cfg(test)]
mod tests {
    extern crate alloc;

    use super::*;
    use crate::arch::x86::boot::compressed::efi::EFI64_LOADER_SIGNATURE;
    use alloc::vec::Vec;

    #[test]
    fn e820_round_trip() {
        let mut bp = BootParams::new();
        bp.set_sentinel();
        bp.set_e820_entry(
            0,
            BootE820Entry {
                base_addr: 0x1000,
                length: 0x2000,
                region_type: 1,
            },
        );
        bp.set_e820_entry(
            1,
            BootE820Entry {
                base_addr: 0x3000,
                length: 0x1000,
                region_type: 2,
            },
        );
        bp.set_e820_entries(2);

        let entries: Vec<_> = bp.e820_iter().collect();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].base_addr, 0x1000);
        assert_eq!(entries[0].region_type, 1);
        assert_eq!(entries[1].region_type, 2);
    }

    #[test]
    fn screen_info_framebuffer_sets_64bit_base() {
        let mut bp = BootParams::new();
        let fb_addr: u64 = 0x1234_5678_9abc_def0;
        bp.set_screen_info_framebuffer(800, 600, 32, 3200, fb_addr);
        let fb = bp.framebuffer_info().unwrap();
        assert_eq!(fb.addr, fb_addr);
        assert_eq!(fb.width, 800);
        assert_eq!(fb.height, 600);
        assert_eq!(fb.depth, 32);
        assert_eq!(fb.pitch, 3200);
    }

    #[test]
    fn setup_header_accessors_round_trip() {
        let mut bp = BootParams::new();
        let efi = EfiInfo {
            efi_loader_signature: *EFI64_LOADER_SIGNATURE,
            efi_systab: 0x89ab_cdef,
            efi_memdesc_size: 40,
            efi_memdesc_version: 1,
            efi_memmap: 0x7654_3210,
            efi_memmap_size: 0x280,
            efi_systab_hi: 0x1234_5678,
            efi_memmap_hi: 0xfedc_ba98,
        };
        bp.set_boot_header_version(0x020f);
        bp.set_acpi_rsdp_addr(0x000f_1234);
        bp.set_loadflags(0x82);
        bp.clear_loadflags(0x02);
        bp.set_setup_data(0x1234_5678_9000);
        bp.set_hardware_subarch(2);
        bp.set_hardware_subarch_data(0xabcd);
        bp.set_ramdisk_image(0x1_2345_6000);
        bp.set_ramdisk_size(0x2_0000_1000);
        bp.set_cmd_line_ptr(0x1_1234_5678);
        bp.set_efi_info(efi);

        assert_eq!(bp.boot_header_version(), 0x020f);
        assert_eq!(bp.acpi_rsdp_addr(), 0x000f_1234);
        assert_eq!(bp.loadflags(), 0x80);
        assert_eq!(bp.setup_data(), 0x1234_5678_9000);
        assert_eq!(bp.hardware_subarch(), 2);
        assert_eq!(bp.hardware_subarch_data(), 0xabcd);
        assert_eq!(bp.ramdisk_image(), 0x1_2345_6000);
        assert_eq!(bp.ramdisk_size(), 0x2_0000_1000);
        assert_eq!(bp.cmd_line_ptr(), 0x1_1234_5678);
        assert_eq!(bp.efi_info(), efi);
    }

    #[test]
    fn screen_info_text_mode_accessors_round_trip() {
        let mut bp = BootParams::new();
        bp.set_screen_orig_video_mode(7);
        bp.set_screen_orig_video_cols(132);
        bp.set_screen_orig_video_lines(43);

        assert_eq!(bp.screen_orig_video_mode(), 7);
        assert_eq!(bp.screen_orig_video_cols(), 132);
        assert_eq!(bp.screen_orig_video_lines(), 43);
    }

    #[test]
    fn boot_params_is_exact_linux_zeropage_size() {
        assert_eq!(core::mem::size_of::<BootParams>(), BOOT_PARAMS_SIZE);
    }
}
