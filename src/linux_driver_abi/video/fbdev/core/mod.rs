//! linux-parity: partial
//! linux-source: vendor/linux/drivers/video/fbdev/core
//! test-origin: linux:vendor/linux/drivers/video/fbdev/core
/// Framebuffer output subsystem — pixel-based text rendering.
///
/// Provides a text console on the linear framebuffer described by Linux
/// `boot_params.screen_info`.  This is the graphical equivalent of the VGA text mode
/// (0xB8000) buffer, using an embedded 8×16 bitmap font to render characters
/// pixel-by-pixel.
///
/// The framebuffer is optional — if the bootloader does not provide one (e.g.,
/// no VBE support), the kernel falls back to VGA text mode.
///
/// Ref: Linux drivers/video/console/fbcon.c — framebuffer console
///      Linux drivers/video/fbdev/ — framebuffer device drivers
pub mod font;
pub mod writer;

use lazy_static::lazy_static;
use spin::Mutex;
use writer::FramebufferWriter;

lazy_static! {
    /// Global framebuffer writer, protected by a spin lock.
    ///
    /// Initialized during boot if `boot_params.screen_info` describes a
    /// linear framebuffer. If None, the framebuffer is not available and the
    /// kernel uses VGA text mode instead.
    pub static ref FB_WRITER: Mutex<Option<FramebufferWriter>> = Mutex::new(None);
}

/// Hardware description of the linear framebuffer set up by the bootloader.
/// Captured at `init()` so the fbdev character device can expose it without
/// reaching back into the `FramebufferWriter` (which is held under a lock the
/// fbdev ioctl path would deadlock against during text-console output).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FramebufferInfo {
    /// Compatibility base used by older call sites.
    pub addr: u64,
    /// Physical base of the aperture reported to fbdev userspace.
    pub phys_addr: u64,
    /// Kernel-visible mapping used for in-kernel drawing and fbdev copies.
    pub kernel_addr: u64,
    pub pitch: u32,
    pub width: u32,
    pub height: u32,
    pub bpp: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SyntheticFramebufferMode {
    pub width: u32,
    pub height: u32,
    pub bpp: u8,
}

impl SyntheticFramebufferMode {
    pub const DEFAULT: Self = Self {
        width: 800,
        height: 600,
        bpp: 32,
    };

    pub fn pitch(self) -> Option<u32> {
        let bytes_per_pixel = match self.bpp {
            24 => 3,
            32 => 4,
            _ => return None,
        };
        self.width.checked_mul(bytes_per_pixel)
    }
}

static mut FB_INFO: Option<FramebufferInfo> = None;

fn parse_synthetic_framebuffer_mode(value: &str) -> Option<SyntheticFramebufferMode> {
    if value == "1" || value.eq_ignore_ascii_case("yes") || value.eq_ignore_ascii_case("on") {
        return Some(SyntheticFramebufferMode::DEFAULT);
    }

    let mut parts = value.split('x');
    let width = parts.next()?.parse::<u32>().ok()?;
    let height = parts.next()?.parse::<u32>().ok()?;
    let bpp = parts.next()?.parse::<u8>().ok()?;
    if parts.next().is_some() {
        return None;
    }

    let mode = SyntheticFramebufferMode { width, height, bpp };
    let pitch = mode.pitch()?;
    checked_framebuffer_size(1, pitch, width, height, bpp)?;
    Some(mode)
}

pub fn synthetic_framebuffer_mode_from_cmdline(cmdline: &str) -> Option<SyntheticFramebufferMode> {
    for token in cmdline.split_whitespace() {
        if token == "lupos.synthetic_fb" {
            return Some(SyntheticFramebufferMode::DEFAULT);
        }
        if let Some(value) = token.strip_prefix("lupos.synthetic_fb=") {
            return parse_synthetic_framebuffer_mode(value);
        }
    }
    None
}

/// Validate framebuffer geometry before any MMIO writes are attempted.
///
/// The console writer only supports direct-color 24-bit BGR and 32-bit BGRX
/// framebuffers.  The bootloader-provided pitch must cover every byte the
/// writer can touch in a row, and the full aperture size must be representable
/// so callers can map exactly the range that will be accessed.
pub fn checked_framebuffer_size(
    addr: u64,
    pitch: u32,
    width: u32,
    height: u32,
    bpp: u8,
) -> Option<u64> {
    if addr == 0 || width == 0 || height == 0 || pitch == 0 {
        return None;
    }

    let bytes_per_pixel = match bpp {
        24 => 3u32,
        32 => 4u32,
        _ => return None,
    };
    let min_pitch = width.checked_mul(bytes_per_pixel)?;
    if pitch < min_pitch {
        return None;
    }

    let size = u64::from(pitch).checked_mul(u64::from(height))?;
    if size == 0 || addr.checked_add(size - 1).is_none() {
        return None;
    }

    Some(size)
}

/// Return the bootloader-provided framebuffer geometry, if one was installed.
pub fn fb_info() -> Option<FramebufferInfo> {
    // Safety: `FB_INFO` is written exactly once during single-threaded boot
    // (`init` runs before SMP brings up other cores) and never mutated again.
    unsafe { FB_INFO }
}

/// Initialize the framebuffer writer from Linux `screen_info` framebuffer data.
///
/// Called during boot if a framebuffer is available.
///
/// # Safety
/// `addr` must be a valid physical framebuffer aperture of at least
/// `pitch * height` bytes.  The kernel will prefer a PAT write-combining
/// ioremap for drawing and keep `addr` as the fbdev physical address.
pub unsafe fn init(addr: u64, pitch: u32, width: u32, height: u32, bpp: u8) -> bool {
    let Some(size) = checked_framebuffer_size(addr, pitch, width, height, bpp) else {
        crate::init::boot_trace::record("fbcon", "invalid framebuffer geometry ignored");
        return false;
    };
    let kernel_addr = unsafe {
        crate::arch::x86::mm::ioremap::ioremap_wc(addr, size)
            .map(|mapping| {
                crate::init::boot_trace::record("fbcon", "write-combining mapping ready");
                mapping.virt
            })
            .unwrap_or(addr)
    };
    unsafe { init_from_kernel_mapping(addr, kernel_addr, pitch, width, height, bpp) }
}

/// Initialize fbcon/fbdev from an already kernel-visible framebuffer mapping.
///
/// # Safety
/// `kernel_addr` must point to writable memory covering `pitch * height` bytes.
pub unsafe fn init_from_kernel_mapping(
    phys_addr: u64,
    kernel_addr: u64,
    pitch: u32,
    width: u32,
    height: u32,
    bpp: u8,
) -> bool {
    if kernel_addr == 0 || checked_framebuffer_size(phys_addr, pitch, width, height, bpp).is_none()
    {
        crate::init::boot_trace::record("fbcon", "invalid framebuffer geometry ignored");
        return false;
    }

    let writer =
        unsafe { FramebufferWriter::new(kernel_addr as *mut u8, pitch, width, height, bpp) };
    let mut fb = FB_WRITER.lock();
    *fb = Some(writer);
    unsafe {
        FB_INFO = Some(FramebufferInfo {
            addr: phys_addr,
            phys_addr,
            kernel_addr,
            pitch,
            width,
            height,
            bpp,
        });
    }
    crate::kernel::console::init_from_pixels(width, height);
    crate::init::boot_trace::record("fbcon", "framebuffer detected");
    // Avoid a full-screen clear during boot: QEMU's framebuffer aperture can
    // be slow to touch, and the boot smoke only needs the console to be live.
    true
}

pub fn init_synthetic(mode: SyntheticFramebufferMode) -> bool {
    let Some(pitch) = mode.pitch() else {
        crate::init::boot_trace::record("fbcon", "invalid synthetic framebuffer mode ignored");
        return false;
    };
    let Some(size) = checked_framebuffer_size(1, pitch, mode.width, mode.height, mode.bpp) else {
        crate::init::boot_trace::record("fbcon", "invalid synthetic framebuffer mode ignored");
        return false;
    };
    let Ok(size) = usize::try_from(size) else {
        crate::init::boot_trace::record("fbcon", "synthetic framebuffer too large");
        return false;
    };

    let ptr = crate::mm::page_alloc::alloc_pages_exact_noprof(
        size,
        crate::mm::page_flags::GFP_KERNEL | crate::mm::page_flags::__GFP_ZERO,
    );
    if ptr.is_null() {
        crate::init::boot_trace::record("fbcon", "synthetic framebuffer allocation failed");
        return false;
    }

    let Some(phys_addr) = crate::arch::x86::mm::paging::virt_to_phys(ptr as u64) else {
        crate::mm::page_alloc::free_pages_exact(ptr, size);
        crate::init::boot_trace::record(
            "fbcon",
            "synthetic framebuffer address translation failed",
        );
        return false;
    };

    let ok = unsafe {
        init_from_kernel_mapping(
            phys_addr,
            ptr as u64,
            pitch,
            mode.width,
            mode.height,
            mode.bpp,
        )
    };
    if ok {
        crate::init::boot_trace::record("fbcon", "synthetic framebuffer ready");
    } else {
        crate::mm::page_alloc::free_pages_exact(ptr, size);
    }
    ok
}

/// Print formatted text to the framebuffer (if available).
#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments<'_>) {
    use core::fmt::Write;
    let mut fb = FB_WRITER.lock();
    if let Some(ref mut writer) = *fb {
        writer.write_fmt(args).unwrap();
    }
}

/// Write raw console bytes to the framebuffer text console.
pub fn write_bytes(bytes: &[u8]) {
    crate::kernel::console::write_visible_bytes(bytes);
}

/// Render a batch of dirty VT rows into the framebuffer backend.
pub fn render_batch(batch: &crate::kernel::console::RenderBatch) {
    let mut fb = FB_WRITER.lock();
    let Some(ref mut writer) = *fb else {
        drop(fb);
        crate::linux_driver_abi::video::console::vgacon::render_batch(batch);
        return;
    };
    let max_rows = batch.rows.min(writer.rows());
    let max_cols = batch.cols.min(writer.cols());
    if let Some(clear) = batch.clear {
        writer.clear_visible_cells(clear.blank, batch.cursor);
    }
    for dirty in &batch.dirty_rows {
        if dirty.row >= max_rows {
            continue;
        }
        for col in 0..max_cols.min(dirty.cells.len()) {
            let cursor = batch.cursor == Some((col, dirty.row));
            writer.render_batch_cell(col, dirty.row, dirty.cells[col], cursor);
        }
    }
}

pub fn refresh_cursor_blink() {
    crate::kernel::console::refresh_cursor_blink();
}

/// Return framebuffer console dimensions as (cols, rows, x pixels, y pixels).
pub fn text_dimensions() -> Option<(u16, u16, u16, u16)> {
    let fb = FB_WRITER.lock();
    fb.as_ref().map(|writer| {
        (
            writer.cols().min(u16::MAX as usize) as u16,
            writer.rows().min(u16::MAX as usize) as u16,
            writer.pixel_width(),
            writer.pixel_height(),
        )
    })
}

/// Print to the framebuffer (no trailing newline).
#[macro_export]
macro_rules! fb_print {
    ($($arg:tt)*) => {
        $crate::linux_driver_abi::video::fbdev::core::_print(format_args!($($arg)*))
    };
}

/// Print to the framebuffer with a trailing newline.
#[macro_export]
macro_rules! fb_println {
    () => { $crate::fb_print!("\n") };
    ($fmt:expr) => { $crate::fb_print!(concat!($fmt, "\n")) };
    ($fmt:expr, $($arg:tt)*) => { $crate::fb_print!(concat!($fmt, "\n"), $($arg)*) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_framebuffer_size_accepts_supported_modes() {
        assert_eq!(
            checked_framebuffer_size(0x1000, 3200, 800, 600, 32),
            Some(1_920_000)
        );
        assert_eq!(
            checked_framebuffer_size(0x1000, 2400, 800, 600, 24),
            Some(1_440_000)
        );
    }

    #[test]
    fn checked_framebuffer_size_rejects_unsafe_modes() {
        assert_eq!(checked_framebuffer_size(0, 3200, 800, 600, 32), None);
        assert_eq!(checked_framebuffer_size(0x1000, 3199, 800, 600, 32), None);
        assert_eq!(checked_framebuffer_size(0x1000, 800, 800, 600, 8), None);
        assert_eq!(checked_framebuffer_size(u64::MAX - 7, 8, 1, 2, 32), None);
    }

    #[test]
    fn parses_synthetic_framebuffer_cmdline_option() {
        assert_eq!(
            synthetic_framebuffer_mode_from_cmdline("quiet lupos.synthetic_fb=800x600x32"),
            Some(SyntheticFramebufferMode {
                width: 800,
                height: 600,
                bpp: 32,
            })
        );
        assert_eq!(
            synthetic_framebuffer_mode_from_cmdline("lupos.synthetic_fb=1"),
            Some(SyntheticFramebufferMode::DEFAULT)
        );
        assert_eq!(
            synthetic_framebuffer_mode_from_cmdline("lupos.synthetic_fb=800x600x16"),
            None
        );
    }
}
