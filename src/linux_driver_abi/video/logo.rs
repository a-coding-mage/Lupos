//! linux-parity: partial
//! linux-source: vendor/linux/drivers/video/logo/logo.c
//! test-origin: linux:vendor/linux/drivers/video/logo/logo.c
//! Linux boot-logo selection.
//!
//! Ref: `vendor/linux/drivers/video/logo/logo.c`

use core::sync::atomic::{AtomicBool, Ordering};

pub const LINUX_LOGO_MONO: i32 = 1;
pub const LINUX_LOGO_VGA16: i32 = 2;
pub const LINUX_LOGO_CLUT224: i32 = 3;
pub const LINUX_LOGO_GRAY256: i32 = 4;

static NOLOGO: AtomicBool = AtomicBool::new(false);
static LOGOS_FREED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LinuxLogoKind {
    Mono,
    Vga16,
    Clut224,
}

impl LinuxLogoKind {
    pub const fn linux_type(self) -> i32 {
        match self {
            Self::Mono => LINUX_LOGO_MONO,
            Self::Vga16 => LINUX_LOGO_VGA16,
            Self::Clut224 => LINUX_LOGO_CLUT224,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LogoConfig {
    pub logo_linux_mono: bool,
    pub logo_linux_vga16: bool,
    pub logo_linux_clut224: bool,
}

impl LogoConfig {
    pub const fn all_linux_logos() -> Self {
        Self {
            logo_linux_mono: true,
            logo_linux_vga16: true,
            logo_linux_clut224: true,
        }
    }

    pub const fn none() -> Self {
        Self {
            logo_linux_mono: false,
            logo_linux_vga16: false,
            logo_linux_clut224: false,
        }
    }
}

pub fn set_nologo(enabled: bool) {
    NOLOGO.store(enabled, Ordering::SeqCst);
}

pub fn nologo() -> bool {
    NOLOGO.load(Ordering::SeqCst)
}

pub fn logos_freed() -> bool {
    LOGOS_FREED.load(Ordering::SeqCst)
}

pub fn fb_logo_late_init() -> i32 {
    LOGOS_FREED.store(true, Ordering::SeqCst);
    0
}

pub fn fb_find_logo(depth: i32) -> Option<LinuxLogoKind> {
    fb_find_logo_with_config(depth, LogoConfig::all_linux_logos())
}

pub fn fb_find_logo_with_config(depth: i32, config: LogoConfig) -> Option<LinuxLogoKind> {
    if nologo() || logos_freed() {
        return None;
    }

    let mut logo = None;

    if config.logo_linux_mono && depth >= 1 {
        logo = Some(LinuxLogoKind::Mono);
    }

    if config.logo_linux_vga16 && depth >= 4 {
        logo = Some(LinuxLogoKind::Vga16);
    }

    if config.logo_linux_clut224 && depth >= 8 {
        logo = Some(LinuxLogoKind::Clut224);
    }

    logo
}

#[cfg(test)]
fn reset_logo_state_for_tests() {
    NOLOGO.store(false, Ordering::SeqCst);
    LOGOS_FREED.store(false, Ordering::SeqCst);
}

#[cfg(test)]
static LOGO_STATE_TEST_LOCK: spin::Mutex<()> = spin::Mutex::new(());

// In the kernel binary, use the pre-generated pixel data.
#[cfg(not(test))]
static LOGO_DATA: &[u8] = include_bytes!(env!("LUPOS_SPLASH_BIN"));

// In host unit-test builds, use an empty placeholder so test compilation succeeds
// without the bare-metal env var being meaningful.
#[cfg(test)]
static LOGO_DATA: &[u8] = &[];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SplashLayout {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

/// Fit the source image inside the framebuffer without changing its aspect
/// ratio.  The remaining pixels are the black letterbox rendered by
/// `fb_show_logo`.
fn splash_layout(
    source_width: usize,
    source_height: usize,
    framebuffer_width: usize,
    framebuffer_height: usize,
) -> Option<SplashLayout> {
    if source_width == 0 || source_height == 0 || framebuffer_width == 0 || framebuffer_height == 0
    {
        return None;
    }

    let source_width_u64 = source_width as u64;
    let source_height_u64 = source_height as u64;
    let framebuffer_width_u64 = framebuffer_width as u64;
    let framebuffer_height_u64 = framebuffer_height as u64;

    let (width, height) =
        if framebuffer_width_u64 * source_height_u64 <= framebuffer_height_u64 * source_width_u64 {
            (
                framebuffer_width,
                ((source_height_u64 * framebuffer_width_u64) / source_width_u64).max(1) as usize,
            )
        } else {
            (
                ((source_width_u64 * framebuffer_height_u64) / source_height_u64).max(1) as usize,
                framebuffer_height,
            )
        };

    Some(SplashLayout {
        x: (framebuffer_width - width) / 2,
        y: (framebuffer_height - height) / 2,
        width,
        height,
    })
}

/// Store one already-packed 32-bit framebuffer pixel in little-endian byte
/// order. Writing all four bytes matters for valid `screen_info` layouts that
/// place a color channel, rather than filler, in bits 24..31.
unsafe fn write_packed_pixel_32(ptr: *mut u8, offset: usize, color: u32) {
    unsafe {
        core::ptr::write_volatile(ptr.add(offset), color as u8);
        core::ptr::write_volatile(ptr.add(offset + 1), (color >> 8) as u8);
        core::ptr::write_volatile(ptr.add(offset + 2), (color >> 16) as u8);
        core::ptr::write_volatile(ptr.add(offset + 3), (color >> 24) as u8);
    }
}

/// Display the Lupos splash on the linear framebuffer at the framebuffer's
/// actual resolution.
///
/// Silently returns if the framebuffer is unavailable (headless boot) or
/// if the pixel data is malformed.
pub fn fb_show_logo() {
    if fb_find_logo(32).is_none() {
        return;
    }

    let Some(fb) = crate::linux_driver_abi::video::fbdev::core::fb_info() else {
        return;
    };
    if LOGO_DATA.len() < 8 || fb.bpp != 32 {
        return;
    }

    let logo_w = u32::from_le_bytes(LOGO_DATA[0..4].try_into().unwrap()) as usize;
    let logo_h = u32::from_le_bytes(LOGO_DATA[4..8].try_into().unwrap()) as usize;
    let Some(expected_len) = logo_w
        .checked_mul(logo_h)
        .and_then(|pixels| pixels.checked_mul(4))
        .and_then(|bytes| bytes.checked_add(8))
    else {
        return;
    };
    if LOGO_DATA.len() < expected_len || logo_w == 0 || logo_h == 0 {
        return;
    }

    let fb_w = fb.width as usize;
    let fb_h = fb.height as usize;
    let Some(layout) = splash_layout(logo_w, logo_h, fb_w, fb_h) else {
        return;
    };

    // Paint every visible framebuffer pixel in one pass.  Besides scaling the
    // splash, this replaces the previous GRUB/menu contents and fbcon's initial
    // top-left cursor cell with a deterministic black letterbox.
    for row in 0..fb_h {
        for col in 0..fb_w {
            let color = if row >= layout.y
                && row < layout.y + layout.height
                && col >= layout.x
                && col < layout.x + layout.width
            {
                let source_row = (row - layout.y) * logo_h / layout.height;
                let source_col = (col - layout.x) * logo_w / layout.width;
                let px_off = 8 + (source_row * logo_w + source_col) * 4;
                u32::from_le_bytes(LOGO_DATA[px_off..px_off + 4].try_into().unwrap())
            } else {
                0
            };
            let color = fb.pixel_format.encode_rgb888(color);
            let offset = row * fb.pitch as usize + col * 4;
            unsafe {
                let ptr = fb.kernel_addr as *mut u8;
                write_packed_pixel_32(ptr, offset, color);
            }
        }
    }

    crate::init::boot_trace::record("logo", "displayed splash on framebuffer");
    crate::log_info!("logo", "displayed splash on framebuffer");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logo_source_contract_matches_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/drivers/video/logo/logo.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/linux_logo.h"
        ));

        assert!(source.contains("static bool nologo;"));
        assert!(source.contains("module_param(nologo, bool, 0);"));
        assert!(source.contains("static bool logos_freed;"));
        assert!(source.contains("static int __init fb_logo_late_init(void)"));
        assert!(source.contains("late_initcall_sync(fb_logo_late_init);"));
        assert!(source.contains("const struct linux_logo * __ref fb_find_logo(int depth)"));
        assert!(source.contains("if (nologo || logos_freed)"));
        assert!(source.contains("#ifdef CONFIG_LOGO_LINUX_MONO"));
        assert!(source.contains("#ifdef CONFIG_LOGO_LINUX_VGA16"));
        assert!(source.contains("#ifdef CONFIG_LOGO_LINUX_CLUT224"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(fb_find_logo);"));

        assert!(header.contains("#define LINUX_LOGO_MONO"));
        assert!(header.contains("#define LINUX_LOGO_VGA16"));
        assert!(header.contains("#define LINUX_LOGO_CLUT224"));
        assert!(header.contains("extern const struct linux_logo *fb_find_logo(int depth);"));
    }

    #[test]
    fn fb_find_logo_selects_highest_configured_logo_for_depth() {
        let _guard = LOGO_STATE_TEST_LOCK.lock();
        reset_logo_state_for_tests();

        assert_eq!(fb_find_logo(0), None);
        assert_eq!(fb_find_logo(1), Some(LinuxLogoKind::Mono));
        assert_eq!(fb_find_logo(3), Some(LinuxLogoKind::Mono));
        assert_eq!(fb_find_logo(4), Some(LinuxLogoKind::Vga16));
        assert_eq!(fb_find_logo(7), Some(LinuxLogoKind::Vga16));
        assert_eq!(fb_find_logo(8), Some(LinuxLogoKind::Clut224));
        assert_eq!(fb_find_logo(32), Some(LinuxLogoKind::Clut224));
    }

    #[test]
    fn fb_find_logo_honors_config_guards_and_late_free_state() {
        let _guard = LOGO_STATE_TEST_LOCK.lock();
        reset_logo_state_for_tests();

        assert_eq!(
            fb_find_logo_with_config(
                8,
                LogoConfig {
                    logo_linux_mono: true,
                    logo_linux_vga16: false,
                    logo_linux_clut224: false,
                },
            ),
            Some(LinuxLogoKind::Mono)
        );
        assert_eq!(
            fb_find_logo_with_config(
                8,
                LogoConfig {
                    logo_linux_mono: false,
                    logo_linux_vga16: true,
                    logo_linux_clut224: false,
                },
            ),
            Some(LinuxLogoKind::Vga16)
        );
        assert_eq!(fb_find_logo_with_config(8, LogoConfig::none()), None);

        set_nologo(true);
        assert_eq!(fb_find_logo(32), None);

        set_nologo(false);
        assert_eq!(fb_logo_late_init(), 0);
        assert_eq!(fb_find_logo(32), None);
        assert!(logos_freed());
    }

    #[test]
    fn logo_type_values_match_linux_header() {
        assert_eq!(LINUX_LOGO_MONO, 1);
        assert_eq!(LINUX_LOGO_VGA16, 2);
        assert_eq!(LINUX_LOGO_CLUT224, 3);
        assert_eq!(LINUX_LOGO_GRAY256, 4);
        assert_eq!(LinuxLogoKind::Mono.linux_type(), LINUX_LOGO_MONO);
        assert_eq!(LinuxLogoKind::Vga16.linux_type(), LINUX_LOGO_VGA16);
        assert_eq!(LinuxLogoKind::Clut224.linux_type(), LINUX_LOGO_CLUT224);
    }

    #[test]
    fn splash_fills_800x600_width_with_aspect_correct_letterbox() {
        assert_eq!(
            splash_layout(800, 450, 800, 600),
            Some(SplashLayout {
                x: 0,
                y: 75,
                width: 800,
                height: 450,
            })
        );
    }

    #[test]
    fn splash_layout_scales_up_and_down_with_the_framebuffer() {
        assert_eq!(
            splash_layout(800, 450, 1920, 1080),
            Some(SplashLayout {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            })
        );
        assert_eq!(
            splash_layout(800, 450, 640, 480),
            Some(SplashLayout {
                x: 0,
                y: 60,
                width: 640,
                height: 360,
            })
        );
    }

    #[test]
    fn splash_layout_rejects_empty_geometry() {
        assert_eq!(splash_layout(0, 450, 800, 600), None);
        assert_eq!(splash_layout(800, 450, 0, 600), None);
    }

    #[test]
    fn packed_pixel_writer_preserves_high_color_channel_byte() {
        let mut pixel = [0u8; 4];
        unsafe { write_packed_pixel_32(pixel.as_mut_ptr(), 0, 0xab12_3456) };
        assert_eq!(pixel, [0x56, 0x34, 0x12, 0xab]);
    }
}
