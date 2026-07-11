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
    /// Published framebuffer metadata. Unlike the former `static mut`, this
    /// remains race-free when a native DRM driver detaches firmware fbdev at
    /// runtime.
    static ref FB_INFO: Mutex<Option<PublishedFramebuffer>> = Mutex::new(None);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FramebufferOrigin {
    Firmware,
    Synthetic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PublishedFramebuffer {
    info: FramebufferInfo,
    origin: FramebufferOrigin,
}

#[cfg(test)]
pub(super) static FRAMEBUFFER_STATE_TEST_LOCK: Mutex<()> = Mutex::new(());

/// One packed-pixel channel from Linux `struct screen_info`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ColorField {
    pub offset: u8,
    pub length: u8,
}

/// Direct-color layout supplied by the firmware in Linux `screen_info`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelFormat {
    pub red: ColorField,
    pub green: ColorField,
    pub blue: ColorField,
    pub reserved: ColorField,
}

impl PixelFormat {
    #[allow(clippy::too_many_arguments)]
    pub const fn from_screen_info(
        red_size: u8,
        red_pos: u8,
        green_size: u8,
        green_pos: u8,
        blue_size: u8,
        blue_pos: u8,
        rsvd_size: u8,
        rsvd_pos: u8,
    ) -> Self {
        Self {
            red: ColorField {
                offset: red_pos,
                length: red_size,
            },
            green: ColorField {
                offset: green_pos,
                length: green_size,
            },
            blue: ColorField {
                offset: blue_pos,
                length: blue_size,
            },
            reserved: ColorField {
                offset: rsvd_pos,
                length: rsvd_size,
            },
        }
    }

    pub const RGB888: Self = Self {
        red: ColorField {
            offset: 16,
            length: 8,
        },
        green: ColorField {
            offset: 8,
            length: 8,
        },
        blue: ColorField {
            offset: 0,
            length: 8,
        },
        reserved: ColorField {
            offset: 0,
            length: 0,
        },
    };

    pub const XRGB8888: Self = Self {
        reserved: ColorField {
            offset: 24,
            length: 8,
        },
        ..Self::RGB888
    };

    pub const fn packed_rgb_for_bpp(bpp: u8) -> Option<Self> {
        match bpp {
            24 => Some(Self::RGB888),
            32 => Some(Self::XRGB8888),
            _ => None,
        }
    }

    /// Validate all channel ranges before they are used for shifts or MMIO.
    /// Linux's simple-framebuffer path obtains this guarantee by matching a
    /// fixed format table; the local fbcon accepts the same packed channel
    /// description directly, so it performs the equivalent bounds checks.
    pub fn is_valid_for_bpp(self, bpp: u8) -> bool {
        if !matches!(bpp, 24 | 32)
            || self.red.length == 0
            || self.green.length == 0
            || self.blue.length == 0
        {
            return false;
        }

        let fields = [self.red, self.green, self.blue, self.reserved];
        let mut masks = [0u32; 4];
        for (index, field) in fields.iter().enumerate() {
            if field.length == 0 {
                continue;
            }
            let end = u16::from(field.offset) + u16::from(field.length);
            if end > u16::from(bpp) || field.length > 32 {
                return false;
            }
            let ones = if field.length == 32 {
                u32::MAX
            } else {
                (1u32 << field.length) - 1
            };
            masks[index] = ones << field.offset;
        }

        for left in 0..masks.len() {
            for right in left + 1..masks.len() {
                if masks[left] & masks[right] != 0 {
                    return false;
                }
            }
        }
        true
    }

    /// Convert the console's canonical `0x00RRGGBB` color into this packed
    /// firmware format. Reserved/alpha bits remain zero, as in Linux fbcon's
    /// true-color pseudo-palette values.
    pub fn encode_rgb888(self, color: u32) -> u32 {
        let red = ((color >> 16) & 0xff) as u8;
        let green = ((color >> 8) & 0xff) as u8;
        let blue = (color & 0xff) as u8;
        encode_channel(red, self.red)
            | encode_channel(green, self.green)
            | encode_channel(blue, self.blue)
    }
}

fn encode_channel(value: u8, field: ColorField) -> u32 {
    if field.length == 0 || field.length > 32 || field.offset >= 32 {
        return 0;
    }
    let max = if field.length == 32 {
        u32::MAX as u64
    } else {
        (1u64 << field.length) - 1
    };
    let scaled = (u64::from(value) * max + 127) / 255;
    (scaled as u32) << field.offset
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
    pub pixel_format: PixelFormat,
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
/// The console writer supports packed direct-color 24-bit and 32-bit
/// framebuffers. The bootloader-provided pitch must cover every byte the
/// writer can touch in a row, and the full aperture size must be representable
/// so callers can map exactly the range that will be accessed. Color-field
/// validation is layered on top by [`checked_framebuffer_mode_size`].
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

    // `FramebufferWriter` exposes at least one text cell and renders every
    // cell as a complete fixed-size glyph.  Accepting a smaller aperture would
    // let that first glyph write past the advertised width or height.
    if width < font::GLYPH_WIDTH as u32 || height < font::GLYPH_HEIGHT as u32 {
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

pub fn checked_framebuffer_mode_size(
    addr: u64,
    pitch: u32,
    width: u32,
    height: u32,
    bpp: u8,
    pixel_format: PixelFormat,
) -> Option<u64> {
    if !pixel_format.is_valid_for_bpp(bpp) {
        return None;
    }
    checked_framebuffer_size(addr, pitch, width, height, bpp)
}

/// Return the bootloader-provided framebuffer geometry, if one was installed.
pub fn fb_info() -> Option<FramebufferInfo> {
    FB_INFO.lock().as_ref().map(|published| published.info)
}

/// Detach the firmware-provided framebuffer before a native DRM driver takes
/// ownership of its aperture.
///
/// This is the local hot-unplug primitive corresponding to Linux
/// `sysfb_disable()` plus the platform-device detach performed by
/// `aperture_remove_conflicting_devices()`. Aperture overlap detection and
/// mapping teardown remain the caller's responsibility.
///
/// Returns `true` when a firmware framebuffer was detached. Synthetic
/// framebuffers are deliberately retained because they do not own firmware
/// display hardware.
pub fn detach_firmware_framebuffer() -> bool {
    // Match initialization's lock order so the writer and its published
    // metadata change as one state transition.
    let mut writer = FB_WRITER.lock();
    let mut published = FB_INFO.lock();
    if !matches!(
        published.as_ref().map(|entry| entry.origin),
        Some(FramebufferOrigin::Firmware)
    ) {
        return false;
    }

    // Stop new console render batches before removing the backend. Any batch
    // already holding the writer lock has completed before we reach this point.
    crate::kernel::console::set_fbcon_enabled(false);
    *writer = None;
    *published = None;
    drop(published);
    drop(writer);

    super::fbdev_set_ready(false);
    crate::init::boot_trace::record("fbcon", "firmware framebuffer detached");
    true
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
    let Some(pixel_format) = PixelFormat::packed_rgb_for_bpp(bpp) else {
        crate::init::boot_trace::record("fbcon", "invalid framebuffer pixel format ignored");
        return false;
    };
    unsafe { init_with_pixel_format(addr, pitch, width, height, bpp, pixel_format) }
}

/// Initialize from firmware geometry plus its exact `screen_info` color masks.
///
/// # Safety
/// The same aperture requirements as [`init`] apply.
pub unsafe fn init_with_pixel_format(
    addr: u64,
    pitch: u32,
    width: u32,
    height: u32,
    bpp: u8,
    pixel_format: PixelFormat,
) -> bool {
    let Some(size) = checked_framebuffer_mode_size(addr, pitch, width, height, bpp, pixel_format)
    else {
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
    unsafe {
        init_from_kernel_mapping_with_pixel_format(
            addr,
            kernel_addr,
            pitch,
            width,
            height,
            bpp,
            pixel_format,
        )
    }
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
    let Some(pixel_format) = PixelFormat::packed_rgb_for_bpp(bpp) else {
        crate::init::boot_trace::record("fbcon", "invalid framebuffer pixel format ignored");
        return false;
    };
    unsafe {
        init_from_kernel_mapping_with_pixel_format(
            phys_addr,
            kernel_addr,
            pitch,
            width,
            height,
            bpp,
            pixel_format,
        )
    }
}

/// Initialize an existing mapping with the firmware's exact channel layout.
///
/// # Safety
/// The same mapping requirements as [`init_from_kernel_mapping`] apply.
#[allow(clippy::too_many_arguments)]
pub unsafe fn init_from_kernel_mapping_with_pixel_format(
    phys_addr: u64,
    kernel_addr: u64,
    pitch: u32,
    width: u32,
    height: u32,
    bpp: u8,
    pixel_format: PixelFormat,
) -> bool {
    unsafe {
        init_from_kernel_mapping_with_origin(
            phys_addr,
            kernel_addr,
            pitch,
            width,
            height,
            bpp,
            pixel_format,
            FramebufferOrigin::Firmware,
        )
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn init_from_kernel_mapping_with_origin(
    phys_addr: u64,
    kernel_addr: u64,
    pitch: u32,
    width: u32,
    height: u32,
    bpp: u8,
    pixel_format: PixelFormat,
    origin: FramebufferOrigin,
) -> bool {
    let Some(size) =
        checked_framebuffer_mode_size(phys_addr, pitch, width, height, bpp, pixel_format)
    else {
        crate::init::boot_trace::record("fbcon", "invalid framebuffer geometry ignored");
        return false;
    };
    if kernel_addr == 0 || kernel_addr.checked_add(size - 1).is_none() {
        crate::init::boot_trace::record("fbcon", "invalid framebuffer geometry ignored");
        return false;
    }

    let writer = unsafe {
        FramebufferWriter::new_with_pixel_format(
            kernel_addr as *mut u8,
            pitch,
            width,
            height,
            bpp,
            pixel_format,
        )
    };
    let mut fb = FB_WRITER.lock();
    *fb = Some(writer);
    *FB_INFO.lock() = Some(PublishedFramebuffer {
        info: FramebufferInfo {
            addr: phys_addr,
            phys_addr,
            kernel_addr,
            pitch,
            width,
            height,
            bpp,
            pixel_format,
        },
        origin,
    });
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

    let Some(pixel_format) = PixelFormat::packed_rgb_for_bpp(mode.bpp) else {
        crate::mm::page_alloc::free_pages_exact(ptr, size);
        crate::init::boot_trace::record("fbcon", "invalid synthetic framebuffer format");
        return false;
    };
    let ok = unsafe {
        init_from_kernel_mapping_with_origin(
            phys_addr,
            ptr as u64,
            pitch,
            mode.width,
            mode.height,
            mode.bpp,
            pixel_format,
            FramebufferOrigin::Synthetic,
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
        // A batch may have passed the console's enabled check immediately
        // before native DRM detached this backend. Do not redirect that stale
        // batch to VGA after ownership has changed.
        if crate::kernel::console::fbcon_enabled() {
            crate::linux_driver_abi::video::console::vgacon::render_batch(batch);
        }
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

    const XBGR8888: PixelFormat = PixelFormat::from_screen_info(8, 0, 8, 8, 8, 16, 8, 24);

    fn reset_published_framebuffer_for_test() {
        *FB_WRITER.lock() = None;
        *FB_INFO.lock() = None;
        super::super::fbdev_set_ready(false);
        crate::kernel::console::set_fbcon_enabled(true);
    }

    #[test]
    fn screen_info_pixel_format_is_validated_and_encodes_channel_positions() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/drivers/firmware/sysfb_simplefb.c"
        ));
        assert!(source.contains("si->red_size == f->red.length"));
        assert!(source.contains("si->blue_pos == f->blue.offset"));

        assert!(PixelFormat::XRGB8888.is_valid_for_bpp(32));
        assert!(XBGR8888.is_valid_for_bpp(32));
        assert_eq!(
            PixelFormat::XRGB8888.encode_rgb888(0x0012_3456),
            0x0012_3456
        );
        assert_eq!(XBGR8888.encode_rgb888(0x0012_3456), 0x0056_3412);

        let blue_in_high_byte = PixelFormat::from_screen_info(8, 0, 8, 8, 8, 24, 8, 16);
        assert!(blue_in_high_byte.is_valid_for_bpp(32));
        assert_eq!(blue_in_high_byte.encode_rgb888(0x0012_3456), 0x5600_3412);
    }

    #[test]
    fn invalid_or_overlapping_screen_info_masks_are_rejected() {
        let overlapping = PixelFormat::from_screen_info(8, 0, 8, 4, 8, 16, 8, 24);
        let outside_storage = PixelFormat::from_screen_info(8, 24, 8, 8, 8, 0, 0, 0);
        let missing_blue = PixelFormat::from_screen_info(8, 16, 8, 8, 0, 0, 8, 24);

        assert!(!overlapping.is_valid_for_bpp(32));
        assert!(!outside_storage.is_valid_for_bpp(24));
        assert!(!missing_blue.is_valid_for_bpp(32));
        assert_eq!(
            checked_framebuffer_mode_size(0x1000, 32, 8, 16, 32, overlapping),
            None
        );
    }

    #[test]
    fn kernel_mapping_end_must_be_representable() {
        let initialized = unsafe {
            init_from_kernel_mapping_with_pixel_format(
                0x1000,
                u64::MAX - 31,
                32,
                8,
                16,
                32,
                PixelFormat::XRGB8888,
            )
        };
        assert!(!initialized);
    }

    #[test]
    fn drm_detach_does_not_remove_synthetic_framebuffer() {
        let _console_guard = crate::kernel::console::TEST_CONSOLE_LOCK.lock();
        let _guard = FRAMEBUFFER_STATE_TEST_LOCK.lock();
        reset_published_framebuffer_for_test();

        let width = font::GLYPH_WIDTH as u32;
        let height = font::GLYPH_HEIGHT as u32;
        let pitch = width * 4;
        let mut pixels = alloc::vec![0u8; (pitch * height) as usize];
        let initialized = unsafe {
            init_from_kernel_mapping_with_origin(
                0x1000,
                pixels.as_mut_ptr() as u64,
                pitch,
                width,
                height,
                32,
                PixelFormat::XRGB8888,
                FramebufferOrigin::Synthetic,
            )
        };

        assert!(initialized);
        assert!(!detach_firmware_framebuffer());
        assert!(fb_info().is_some());
        assert!(FB_WRITER.lock().is_some());
        assert!(crate::kernel::console::fbcon_enabled());

        reset_published_framebuffer_for_test();
    }

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
    fn checked_framebuffer_size_requires_one_complete_glyph() {
        assert_eq!(checked_framebuffer_size(0x1000, 7 * 4, 7, 16, 32), None);
        assert_eq!(checked_framebuffer_size(0x1000, 8 * 4, 8, 15, 32), None);
        assert_eq!(
            checked_framebuffer_size(0x1000, 8 * 4, 8, 16, 32),
            Some(8 * 4 * 16)
        );
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
