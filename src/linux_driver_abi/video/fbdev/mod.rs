//! linux-parity: partial
//! linux-source: vendor/linux/drivers/video/fbdev
//! test-origin: linux:vendor/linux/drivers/video/fbdev
//! Legacy framebuffer character device — `/dev/fb0`.
//!
//! Wraps the Linux `boot_params.screen_info` linear framebuffer captured by
//! `framebuffer::init` and exposes it through the canonical Linux fbdev ABI. This is what
//! `xf86-video-fbdev` (X.Org) and Weston's `fbdev-backend.so` open.
//!
//! References:
//!   - `vendor/linux/drivers/video/fbdev/core/fbmem.c`
//!   - `vendor/linux/include/uapi/linux/fb.h` — ABI structs + ioctl numbers

extern crate alloc;

pub mod core;
pub use core::detach_firmware_framebuffer;

use ::core::mem::size_of;
use ::core::sync::atomic::{AtomicBool, Ordering};

use crate::fs::ops::{FileOps, IoctlFn, MmapFn, PollFn};
use crate::fs::types::FileRef;
use crate::include::uapi::errno::{EFAULT, EFBIG, EINVAL, ENODEV, ENOTTY};
use crate::linux_driver_abi::video::fbdev::core::{FramebufferInfo, fb_info};
use crate::mm::mm_types::VmAreaStruct;

// ── fbdev ioctl numbers — `include/uapi/linux/fb.h` ───────────────────────────

pub const FBIOGET_VSCREENINFO: u32 = 0x4600;
pub const FBIOPUT_VSCREENINFO: u32 = 0x4601;
pub const FBIOGET_FSCREENINFO: u32 = 0x4602;
pub const FBIOGETCMAP: u32 = 0x4604;
pub const FBIOPUTCMAP: u32 = 0x4605;
pub const FBIOPAN_DISPLAY: u32 = 0x4606;
pub const FBIO_WAITFORVSYNC: u32 = 0x4620;
pub const FBIOBLANK: u32 = 0x4611;

// ── fb_var_screeninfo + fb_fix_screeninfo — ABI-pinned ────────────────────────

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct FbBitfield {
    pub offset: u32,
    pub length: u32,
    pub msb_right: u32,
}

/// `struct fb_var_screeninfo` — variable framebuffer state.
///
/// Layout from `include/uapi/linux/fb.h:130`. Padded to the upstream size so
/// userspace passing a full-size buffer doesn't overflow our struct.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct FbVarScreeninfo {
    pub xres: u32,
    pub yres: u32,
    pub xres_virtual: u32,
    pub yres_virtual: u32,
    pub xoffset: u32,
    pub yoffset: u32,
    pub bits_per_pixel: u32,
    pub grayscale: u32,
    pub red: FbBitfield,
    pub green: FbBitfield,
    pub blue: FbBitfield,
    pub transp: FbBitfield,
    pub nonstd: u32,
    pub activate: u32,
    pub height: u32,
    pub width: u32,
    pub accel_flags: u32,
    pub pixclock: u32,
    pub left_margin: u32,
    pub right_margin: u32,
    pub upper_margin: u32,
    pub lower_margin: u32,
    pub hsync_len: u32,
    pub vsync_len: u32,
    pub sync: u32,
    pub vmode: u32,
    pub rotate: u32,
    pub colorspace: u32,
    pub reserved: [u32; 4],
}

const FB_TYPE_PACKED_PIXELS: u32 = 0;
const FB_VISUAL_TRUECOLOR: u32 = 2;
const FB_ACCEL_NONE: u32 = 0;

/// `struct fb_fix_screeninfo` — fixed framebuffer state.
///
/// Layout from `include/uapi/linux/fb.h:182`.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct FbFixScreeninfo {
    pub id: [u8; 16],
    pub smem_start: u64,
    pub smem_len: u32,
    pub type_: u32,
    pub type_aux: u32,
    pub visual: u32,
    pub xpanstep: u16,
    pub ypanstep: u16,
    pub ywrapstep: u16,
    pub line_length: u32,
    pub mmio_start: u64,
    pub mmio_len: u32,
    pub accel: u32,
    pub capabilities: u16,
    pub reserved: [u16; 2],
}

impl Default for FbFixScreeninfo {
    fn default() -> Self {
        Self {
            id: [0u8; 16],
            smem_start: 0,
            smem_len: 0,
            type_: FB_TYPE_PACKED_PIXELS,
            type_aux: 0,
            visual: FB_VISUAL_TRUECOLOR,
            xpanstep: 0,
            ypanstep: 0,
            ywrapstep: 0,
            line_length: 0,
            mmio_start: 0,
            mmio_len: 0,
            accel: FB_ACCEL_NONE,
            capabilities: 0,
            reserved: [0u16; 2],
        }
    }
}

// ── Driver state ──────────────────────────────────────────────────────────────

static FBDEV_READY: AtomicBool = AtomicBool::new(false);

pub(super) fn fbdev_set_ready(ready: bool) {
    FBDEV_READY.store(ready, Ordering::Release);
}

/// Initialize the fbdev character device.  Returns `true` if the bootloader
/// provided a framebuffer to expose.
pub fn fbdev_init() -> bool {
    if fb_info().is_some() {
        fbdev_set_ready(true);
        true
    } else {
        false
    }
}

fn fb_info_required() -> Result<FramebufferInfo, i32> {
    fb_info().ok_or(ENODEV)
}

fn build_var_screeninfo(info: FramebufferInfo) -> FbVarScreeninfo {
    // Legacy Linux vesafb/efifb expose the screen_info `rsvd_*` field through
    // fb_var_screeninfo.transp. This intentionally differs from the DRM/sysfb
    // `screen_info_pixel_format()` helper, which describes alpha as absent.
    let bitfield = |field: core::ColorField| FbBitfield {
        offset: field.offset as u32,
        length: field.length as u32,
        msb_right: 0,
    };
    FbVarScreeninfo {
        xres: info.width,
        yres: info.height,
        xres_virtual: info.width,
        yres_virtual: info.height,
        xoffset: 0,
        yoffset: 0,
        bits_per_pixel: info.bpp as u32,
        grayscale: 0,
        red: bitfield(info.pixel_format.red),
        green: bitfield(info.pixel_format.green),
        blue: bitfield(info.pixel_format.blue),
        transp: bitfield(info.pixel_format.reserved),
        nonstd: 0,
        activate: 0,
        height: u32::MAX,
        width: u32::MAX,
        accel_flags: 0,
        pixclock: 0,
        left_margin: 0,
        right_margin: 0,
        upper_margin: 0,
        lower_margin: 0,
        hsync_len: 0,
        vsync_len: 0,
        sync: 0,
        vmode: 0,
        rotate: 0,
        colorspace: 0,
        reserved: [0u32; 4],
    }
}

fn build_fix_screeninfo(info: FramebufferInfo) -> FbFixScreeninfo {
    let mut fix = FbFixScreeninfo::default();
    let id = b"lupos-fb\0\0\0\0\0\0\0\0";
    fix.id.copy_from_slice(id);
    fix.smem_start = info.phys_addr;
    fix.smem_len = info.pitch.saturating_mul(info.height);
    fix.line_length = info.pitch;
    fix
}

// ── FileOps ───────────────────────────────────────────────────────────────────

fn fbdev_read(_file: &FileRef, buf: &mut [u8], pos: &mut u64) -> Result<usize, i32> {
    let info = fb_info_required()?;
    let total = info.pitch.saturating_mul(info.height) as u64;
    let start = *pos;
    if start >= total {
        return Ok(0);
    }
    let take = ::core::cmp::min(buf.len() as u64, total - start) as usize;
    unsafe {
        let src = (info.kernel_addr as *const u8).add(start as usize);
        ::core::ptr::copy_nonoverlapping(src, buf.as_mut_ptr(), take);
    }
    *pos += take as u64;
    Ok(take)
}

fn fbdev_write(_file: &FileRef, buf: &[u8], pos: &mut u64) -> Result<usize, i32> {
    let info = fb_info_required()?;
    let total = info.pitch.saturating_mul(info.height) as u64;
    let start = *pos;
    let take = fbdev_write_count(total, start, buf.len())?;
    unsafe {
        let dst = (info.kernel_addr as *mut u8).add(start as usize);
        ::core::ptr::copy_nonoverlapping(buf.as_ptr(), dst, take);
    }
    *pos += take as u64;
    Ok(take)
}

fn fbdev_write_count(total: u64, start: u64, count: usize) -> Result<usize, i32> {
    if start > total {
        return Err(EFBIG);
    }
    if count == 0 {
        return Ok(0);
    }
    if start == total {
        return Err(crate::include::uapi::errno::ENOSPC);
    }
    Ok(::core::cmp::min(count as u64, total - start) as usize)
}

fn fbdev_seek_target(current: u64, total: u64, off: i64, whence: i32) -> Result<u64, i32> {
    let base = match whence {
        0 /* SEEK_SET */ => 0i128,
        1 /* SEEK_CUR */ => current as i128,
        2 /* SEEK_END */ => total as i128,
        _ => return Err(EINVAL),
    };
    let target = base + off as i128;
    if !(0..=u64::MAX as i128).contains(&target) {
        return Err(EINVAL);
    }
    Ok(target as u64)
}

fn fbdev_llseek(file: &FileRef, off: i64, whence: i32) -> Result<u64, i32> {
    let info = fb_info_required()?;
    let total = info.pitch.saturating_mul(info.height) as u64;
    let mut pos = file.pos.lock();
    let target = fbdev_seek_target(*pos, total, off, whence)?;
    *pos = target;
    Ok(target)
}

fn fbdev_poll(_file: &FileRef, _table: Option<&mut crate::fs::select::PollTable>) -> u32 {
    fbdev_poll_mask()
}

fn fbdev_poll_mask() -> u32 {
    use crate::fs::eventpoll::{EPOLLERR, EPOLLHUP, EPOLLIN, EPOLLOUT};
    if fb_info().is_none() {
        // poll has no errno return channel; report hot-unplug through the
        // standard error/hangup readiness bits instead.
        EPOLLERR | EPOLLHUP
    } else {
        // Framebuffer is always writable; never blocks for read either.
        EPOLLIN | EPOLLOUT
    }
}

fn fbdev_ioctl(_file: &FileRef, cmd: u32, arg: u64) -> Result<i64, i32> {
    let info = fb_info_required()?;
    match cmd {
        FBIOGET_VSCREENINFO => {
            if arg == 0 {
                return Err(EFAULT);
            }
            let var = build_var_screeninfo(info);
            let not_copied = unsafe {
                crate::arch::x86::kernel::uaccess::copy_to_user(
                    arg as *mut u8,
                    &var as *const FbVarScreeninfo as *const u8,
                    size_of::<FbVarScreeninfo>(),
                )
            };
            if not_copied == 0 { Ok(0) } else { Err(EFAULT) }
        }
        FBIOPUT_VSCREENINFO => {
            // We support exactly one mode: the bootloader-provided geometry.
            // Accept a request that matches it (a no-op, incl. the driver's
            // FB_ACTIVATE_TEST probe) and reject anything else with -EINVAL,
            // mirroring a real fixed-mode fbdev `fb_set_var`/`check_var`.
            //
            // Unconditionally returning -EINVAL breaks `xf86-video-fbdev`, whose
            // `fbdevHWModeInit` treats a failed `FBIOPUT_VSCREENINFO` for the
            // native mode as fatal ("mode initialization failed" →
            // "AddScreen/ScreenInit failed").
            if arg == 0 {
                return Err(EFAULT);
            }
            let mut req = FbVarScreeninfo::default();
            let not_copied = unsafe {
                crate::arch::x86::kernel::uaccess::copy_from_user(
                    &mut req as *mut FbVarScreeninfo as *mut u8,
                    arg as *const u8,
                    size_of::<FbVarScreeninfo>(),
                )
            };
            if not_copied != 0 {
                return Err(EFAULT);
            }
            // The visible resolution and pixel depth must match; virtual size
            // may be equal or larger (the X server rounds the pitch up), and we
            // ignore timing/margin fields we don't model.
            if req.xres == info.width
                && req.yres == info.height
                && req.bits_per_pixel == info.bpp as u32
                && req.xres_virtual >= info.width
                && req.yres_virtual >= info.height
            {
                Ok(0)
            } else {
                Err(EINVAL)
            }
        }
        FBIOGET_FSCREENINFO => {
            if arg == 0 {
                return Err(EFAULT);
            }
            let fix = build_fix_screeninfo(info);
            let not_copied = unsafe {
                crate::arch::x86::kernel::uaccess::copy_to_user(
                    arg as *mut u8,
                    &fix as *const FbFixScreeninfo as *const u8,
                    size_of::<FbFixScreeninfo>(),
                )
            };
            if not_copied == 0 { Ok(0) } else { Err(EFAULT) }
        }
        FBIOPAN_DISPLAY | FBIOBLANK | FBIO_WAITFORVSYNC => {
            // No vsync timing; pan-display is a no-op for a single-buffer
            // device; blanking isn't wired to a real backlight.
            Ok(0)
        }
        FBIOGETCMAP | FBIOPUTCMAP => Ok(0),
        _ => Err(ENOTTY),
    }
}

fn fbdev_mmap(_file: &FileRef, vma: &mut VmAreaStruct) -> Result<(), i32> {
    let len =
        usize::try_from(vma.vm_end.checked_sub(vma.vm_start).ok_or(EINVAL)?).map_err(|_| EINVAL)?;
    let info = fb_info_required()?;
    let fix = build_fix_screeninfo(info);

    // fb_io_mmap() places the optional MMIO register aperture immediately
    // after the page-aligned framebuffer aperture in the fbdev mmap-offset
    // namespace. The firmware backends currently published by Lupos do not
    // advertise MMIO, but retaining this split is required for the generic
    // helper's exact offset and validation semantics.
    let mmio_pgoff = fbdev_mmio_pgoff(fix.smem_start, fix.smem_len)?;
    let (start, region_len) = if vma.vm_pgoff >= mmio_pgoff {
        if build_var_screeninfo(info).accel_flags != 0 {
            return Err(EINVAL);
        }
        vma.vm_pgoff = vma.vm_pgoff.checked_sub(mmio_pgoff).ok_or(EINVAL)?;
        (fix.mmio_start, fix.mmio_len)
    } else {
        (fix.smem_start, fix.smem_len)
    };
    let phys = fbdev_region_mmap_phys(start, region_len, len, vma.vm_pgoff)?;

    // Linux fb_io_mmap() recomputes the protection from vm_flags, applies the
    // architecture framebuffer cache mode, then vm_iomap_memory() establishes
    // the raw-PFN mapping and its VMA flags.
    let prot = crate::mm::pgprot::vm_get_page_prot(vma.vm_flags);
    vma.vm_page_prot =
        crate::arch::x86::mm::paging::pgprot_val(crate::arch::x86::video::pgprot_framebuffer(
            crate::arch::x86::mm::paging::__pgprot(prot),
            vma.vm_start as usize,
            vma.vm_end as usize,
            start as usize,
        ));
    crate::mm::fault::prepare_lupos_device_pfn_mapping(vma, phys);

    // The validated physical mapping is retained in the VMA. Subsequent page
    // faults only materialize its PTEs; they do not re-enter this callback.
    Ok(())
}

/// Return the mmap-page offset at which `fb_io_mmap()` switches from the
/// framebuffer aperture to the optional MMIO aperture.
fn fbdev_mmio_pgoff(smem_start: u64, smem_len: u32) -> Result<u64, i32> {
    const PAGE_SIZE: u64 = 4096;
    const PAGE_MASK: u64 = PAGE_SIZE - 1;

    let bytes = (smem_start & PAGE_MASK)
        .checked_add(u64::from(smem_len))
        .ok_or(EINVAL)?;
    bytes
        .checked_add(PAGE_MASK)
        .map(|value| (value & !PAGE_MASK) >> 12)
        .ok_or(EINVAL)
}

/// Apply Linux `vm_iomap_memory()`'s `__simple_ioremap_prep()` calculation.
/// The retained address may include the resource's within-page offset; the
/// lazy fault path extracts exactly the same PFN that Linux passes to
/// `io_remap_pfn_range()`.
fn fbdev_region_mmap_phys(start: u64, region_len: u32, len: usize, pgoff: u64) -> Result<u64, i32> {
    const PAGE_SHIFT: u32 = 12;
    const PAGE_SIZE: u64 = 1 << PAGE_SHIFT;
    const PAGE_MASK: u64 = PAGE_SIZE - 1;

    let vm_len = u64::try_from(len).map_err(|_| EINVAL)?;
    if vm_len == 0 || vm_len & PAGE_MASK != 0 {
        return Err(EINVAL);
    }

    let region_len = u64::from(region_len);
    // __simple_ioremap_prep() first rejects a wrapped physical resource.
    start.checked_add(region_len).ok_or(EINVAL)?;
    let size = region_len.checked_add(start & PAGE_MASK).ok_or(EINVAL)?;
    let pages = size
        .checked_add(PAGE_MASK)
        .map(|value| value >> PAGE_SHIFT)
        .ok_or(EINVAL)?;
    let base_pfn = start >> PAGE_SHIFT;
    base_pfn.checked_add(pages).ok_or(EINVAL)?;

    if pgoff > pages {
        return Err(EINVAL);
    }
    base_pfn.checked_add(pgoff).ok_or(EINVAL)?;
    let remaining_pages = pages - pgoff;
    if (vm_len >> PAGE_SHIFT) > remaining_pages {
        return Err(EINVAL);
    }

    start
        .checked_add(pgoff.checked_shl(PAGE_SHIFT).ok_or(EINVAL)?)
        .ok_or(EINVAL)
}

/// Framebuffer-only wrapper retained for the existing vendor-derived range
/// checks. `fbdev_mmap()` uses the region form above after performing
/// `fb_io_mmap()`'s framebuffer/MMIO selection.
fn fbdev_mmap_phys(info: FramebufferInfo, len: usize, off: u64) -> Result<u64, i32> {
    const PAGE_MASK: u64 = 4095;
    if off & PAGE_MASK != 0 {
        return Err(EINVAL);
    }
    let smem_len = info.pitch.saturating_mul(info.height);
    fbdev_region_mmap_phys(info.phys_addr, smem_len, len, off >> 12)
}

pub const FBDEV_FILE_OPS: FileOps = FileOps {
    name: "fbdev",
    read: Some(fbdev_read),
    write: Some(fbdev_write),
    llseek: Some(fbdev_llseek),
    fsync: None,
    poll: Some(fbdev_poll as PollFn),
    ioctl: Some(fbdev_ioctl as IoctlFn),
    mmap: Some(fbdev_mmap as MmapFn),
    release: None,
    readdir: None,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn var_screeninfo_for_xrgb8888_matches_xres_yres() {
        let info = FramebufferInfo {
            addr: 0xfd00_0000,
            phys_addr: 0xfd00_0000,
            kernel_addr: 0xfd00_0000,
            pitch: 1280 * 4,
            width: 1280,
            height: 800,
            bpp: 32,
            pixel_format: core::PixelFormat::XRGB8888,
        };
        let var = build_var_screeninfo(info);
        assert_eq!(var.xres, 1280);
        assert_eq!(var.yres, 800);
        assert_eq!(var.bits_per_pixel, 32);
        assert_eq!(var.red.offset, 16);
        assert_eq!(var.green.offset, 8);
        assert_eq!(var.blue.offset, 0);
        assert_eq!(var.transp.offset, 24);
        assert_eq!(var.transp.length, 8);
    }

    #[test]
    fn var_screeninfo_preserves_firmware_channel_layout() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/drivers/video/fbdev/efifb.c"
        ));
        assert!(source.contains("efifb_defined.transp.offset = si->rsvd_pos;"));
        assert!(source.contains("efifb_defined.transp.length = si->rsvd_size;"));

        let info = FramebufferInfo {
            addr: 0xfd00_0000,
            phys_addr: 0xfd00_0000,
            kernel_addr: 0xfd00_0000,
            pitch: 1280 * 4,
            width: 1280,
            height: 800,
            bpp: 32,
            pixel_format: core::PixelFormat::from_screen_info(8, 0, 8, 8, 8, 16, 8, 24),
        };

        let var = build_var_screeninfo(info);
        assert_eq!((var.red.offset, var.red.length), (0, 8));
        assert_eq!((var.green.offset, var.green.length), (8, 8));
        assert_eq!((var.blue.offset, var.blue.length), (16, 8));
        assert_eq!((var.transp.offset, var.transp.length), (24, 8));
    }

    #[test]
    fn fix_screeninfo_carries_smem_start_and_length() {
        let info = FramebufferInfo {
            addr: 0xfd00_0000,
            phys_addr: 0xfd00_0000,
            kernel_addr: 0xffff_fd00_fd00_0000,
            pitch: 1280 * 4,
            width: 1280,
            height: 800,
            bpp: 32,
            pixel_format: core::PixelFormat::XRGB8888,
        };
        let fix = build_fix_screeninfo(info);
        assert_eq!(fix.smem_start, 0xfd00_0000);
        assert_eq!(fix.smem_len, 1280 * 4 * 800);
        assert_eq!(fix.line_length, 1280 * 4);
        assert!(fix.id.starts_with(b"lupos-fb"));
    }

    #[test]
    fn fbdev_init_returns_false_when_no_framebuffer() {
        let _guard = core::FRAMEBUFFER_STATE_TEST_LOCK.lock();
        // No framebuffer was registered in the host test process, so init
        // should report unavailable.
        assert!(!fbdev_init());
    }

    #[test]
    fn drm_handoff_detaches_firmware_fbdev_and_operations_return_enodev() {
        let _console_guard = crate::kernel::console::TEST_CONSOLE_LOCK.lock();
        let _guard = core::FRAMEBUFFER_STATE_TEST_LOCK.lock();
        crate::kernel::console::set_fbcon_enabled(true);
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/drivers/video/aperture.c"
        ));
        assert!(source.contains("sysfb_disable(NULL);"));
        assert!(source.contains("platform_device_unregister(pdev);"));

        let width = core::font::GLYPH_WIDTH as u32;
        let height = core::font::GLYPH_HEIGHT as u32;
        let pitch = width * 4;
        let mut pixels = alloc::vec![0u8; (pitch * height) as usize];
        let initialized = unsafe {
            core::init_from_kernel_mapping_with_pixel_format(
                0x2000,
                pixels.as_mut_ptr() as u64,
                pitch,
                width,
                height,
                32,
                core::PixelFormat::XRGB8888,
            )
        };

        assert!(initialized);
        assert!(fbdev_init());
        assert!(FBDEV_READY.load(Ordering::Acquire));
        assert!(fb_info_required().is_ok());

        assert!(detach_firmware_framebuffer());
        assert!(!FBDEV_READY.load(Ordering::Acquire));
        assert_eq!(fb_info_required(), Err(ENODEV));
        assert!(core::FB_WRITER.lock().is_none());
        assert!(!crate::kernel::console::fbcon_enabled());
        assert_eq!(
            fbdev_poll_mask(),
            crate::fs::eventpoll::EPOLLERR | crate::fs::eventpoll::EPOLLHUP
        );
        assert!(!detach_firmware_framebuffer());

        // Do not leak the detach state into unrelated console tests.
        crate::kernel::console::set_fbcon_enabled(true);
    }

    #[test]
    fn fbdev_write_bounds_match_linux_fb_io_write() {
        assert_eq!(fbdev_write_count(100, 101, 0), Err(EFBIG));
        assert_eq!(fbdev_write_count(100, 100, 0), Ok(0));
        assert_eq!(
            fbdev_write_count(100, 100, 1),
            Err(crate::include::uapi::errno::ENOSPC)
        );
        assert_eq!(fbdev_write_count(100, 90, 20), Ok(10));
    }

    #[test]
    fn fbdev_seek_supports_set_cur_and_end_and_rejects_underflow() {
        assert_eq!(fbdev_seek_target(40, 100, 7, 0), Ok(7));
        assert_eq!(fbdev_seek_target(40, 100, 7, 1), Ok(47));
        assert_eq!(fbdev_seek_target(40, 100, -7, 2), Ok(93));
        assert_eq!(fbdev_seek_target(3, 100, -4, 1), Err(EINVAL));
        assert_eq!(fbdev_seek_target(3, 100, 0, 99), Err(EINVAL));
    }

    #[test]
    fn fbdev_mmap_rejects_vma_beyond_page_aligned_aperture() {
        let info = FramebufferInfo {
            addr: 0x1000_0123,
            phys_addr: 0x1000_0123,
            kernel_addr: 0xffff_8000_1000_0123,
            pitch: 1000,
            width: 250,
            height: 5,
            bpp: 32,
            pixel_format: core::PixelFormat::XRGB8888,
        };

        // 0x123 bytes into the first page + 5000 aperture bytes occupies two
        // complete pages, exactly as __simple_ioremap_prep computes it.
        assert_eq!(fbdev_mmap_phys(info, 8192, 0), Ok(info.phys_addr));
        assert_eq!(fbdev_mmap_phys(info, 4096, 4096), Ok(info.phys_addr + 4096));
        assert_eq!(fbdev_mmap_phys(info, 8193, 0), Err(EINVAL));
        assert_eq!(fbdev_mmap_phys(info, 4097, 4096), Err(EINVAL));
        assert_eq!(fbdev_mmap_phys(info, 4096, 8192), Err(EINVAL));
        assert_eq!(fbdev_mmap_phys(info, 4096, 1), Err(EINVAL));
    }
}
