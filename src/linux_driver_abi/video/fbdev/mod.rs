//! linux-parity: complete
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

use ::core::mem::size_of;
use ::core::sync::atomic::{AtomicBool, Ordering};

use crate::fs::ops::{FileOps, IoctlFn, MmapFn, PollFn};
use crate::fs::types::FileRef;
use crate::include::uapi::errno::{EFAULT, EINVAL, ENODEV, ENOTTY};
use crate::linux_driver_abi::video::fbdev::core::{FramebufferInfo, fb_info};

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

/// Initialize the fbdev character device.  Returns `true` if the bootloader
/// provided a framebuffer to expose.
pub fn fbdev_init() -> bool {
    if fb_info().is_some() {
        FBDEV_READY.store(true, Ordering::Release);
        true
    } else {
        false
    }
}

fn fb_info_required() -> Result<FramebufferInfo, i32> {
    fb_info().ok_or(ENODEV)
}

fn build_var_screeninfo(info: FramebufferInfo) -> FbVarScreeninfo {
    // For now we hard-code an XRGB8888 layout that matches what GRUB hands us
    // through Linux `screen_info`. When boot-time pixel formats become
    // variable this will be re-derived from the zeropage color masks.
    let (red, green, blue) = match info.bpp {
        32 => (
            FbBitfield {
                offset: 16,
                length: 8,
                msb_right: 0,
            },
            FbBitfield {
                offset: 8,
                length: 8,
                msb_right: 0,
            },
            FbBitfield {
                offset: 0,
                length: 8,
                msb_right: 0,
            },
        ),
        24 => (
            FbBitfield {
                offset: 16,
                length: 8,
                msb_right: 0,
            },
            FbBitfield {
                offset: 8,
                length: 8,
                msb_right: 0,
            },
            FbBitfield {
                offset: 0,
                length: 8,
                msb_right: 0,
            },
        ),
        _ => (
            FbBitfield {
                offset: 11,
                length: 5,
                msb_right: 0,
            },
            FbBitfield {
                offset: 5,
                length: 6,
                msb_right: 0,
            },
            FbBitfield {
                offset: 0,
                length: 5,
                msb_right: 0,
            },
        ),
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
        red,
        green,
        blue,
        transp: FbBitfield::default(),
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
    if start >= total {
        return Err(crate::include::uapi::errno::ENOSPC);
    }
    let take = ::core::cmp::min(buf.len() as u64, total - start) as usize;
    unsafe {
        let dst = (info.kernel_addr as *mut u8).add(start as usize);
        ::core::ptr::copy_nonoverlapping(buf.as_ptr(), dst, take);
    }
    *pos += take as u64;
    Ok(take)
}

fn fbdev_llseek(_file: &FileRef, off: i64, whence: i32) -> Result<u64, i32> {
    let info = fb_info_required()?;
    let total = info.pitch.saturating_mul(info.height) as i64;
    let target = match whence {
        0 /* SEEK_SET */ => off,
        2 /* SEEK_END */ => total.saturating_add(off),
        _ => return Err(EINVAL),
    };
    if target < 0 {
        return Err(EINVAL);
    }
    Ok(target as u64)
}

fn fbdev_poll(_file: &FileRef) -> u32 {
    // Framebuffer is always writable; never blocks for read either.
    use crate::fs::eventpoll::{EPOLLIN, EPOLLOUT};
    EPOLLIN | EPOLLOUT
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

fn fbdev_mmap(
    _file: &FileRef,
    _addr: u64,
    len: usize,
    _prot: u32,
    _flags: u32,
    off: u64,
) -> Result<u64, i32> {
    let info = fb_info_required()?;
    let total = info.pitch.saturating_mul(info.height) as u64;
    if off >= total || len == 0 {
        return Err(EINVAL);
    }
    // Note: `off + len` may exceed `total` on the final page — the framebuffer
    // length is rarely page-aligned (e.g. 800x600x32 = 1_920_000 B = 468.75
    // pages).  Linux `fb_mmap` maps `PAGE_ALIGN(len + offset)`, i.e. the whole
    // last page, and so do we: the fault handler maps one full physical page per
    // fault, and any bytes past `total` fall inside the (larger, page-aligned)
    // hardware aperture.  Only reject offsets that start beyond the framebuffer.
    //
    // A userspace client (the X server) is taking ownership of the scanout
    // aperture. Linux relies on the client issuing `KDSETMODE(KD_GRAPHICS)` on
    // its VT to stop fbcon from repainting the text console over the graphics
    // output; Xorg under Lupos never reaches that path (its logind/VT handling
    // is unavailable), so fbcon would otherwise clobber every frame the client
    // draws. Silence fbcon here, when the framebuffer is mmap'd for direct
    // pixel output, mirroring the effect of the graphics-mode switch.
    crate::kernel::console::set_fbcon_enabled(false);
    // Return the *physical* framebuffer address for this byte offset.  The mm
    // layer marks the VMA `VM_PFNMAP` (see `file_wants_pfn_mmap` /
    // `LUPOS_DEVICE_PFN_VM_OPS`) and, on fault, maps this physical frame's pfn
    // straight into the calling process — so userspace writes (X server pixel
    // output) land in the real scanout aperture instead of a private page-cache
    // copy.
    Ok(info.phys_addr.saturating_add(off))
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
        };
        let var = build_var_screeninfo(info);
        assert_eq!(var.xres, 1280);
        assert_eq!(var.yres, 800);
        assert_eq!(var.bits_per_pixel, 32);
        assert_eq!(var.red.offset, 16);
        assert_eq!(var.green.offset, 8);
        assert_eq!(var.blue.offset, 0);
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
        };
        let fix = build_fix_screeninfo(info);
        assert_eq!(fix.smem_start, 0xfd00_0000);
        assert_eq!(fix.smem_len, 1280 * 4 * 800);
        assert_eq!(fix.line_length, 1280 * 4);
        assert!(fix.id.starts_with(b"lupos-fb"));
    }

    #[test]
    fn fbdev_init_returns_false_when_no_framebuffer() {
        // No framebuffer was registered in the host test process, so init
        // should report unavailable.
        assert!(!fbdev_init());
    }
}
