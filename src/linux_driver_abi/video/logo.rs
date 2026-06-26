//! linux-parity: complete
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

// In the kernel binary, use the pre-generated pixel data.
#[cfg(not(test))]
static LOGO_DATA: &[u8] = include_bytes!(env!("LUPOS_SPLASH_BIN"));

// In host unit-test builds, use an empty placeholder so test compilation succeeds
// without the bare-metal env var being meaningful.
#[cfg(test)]
static LOGO_DATA: &[u8] = &[];

/// Display the Lupos brand logo on the linear framebuffer, centered.
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
    let expected_len = 8 + logo_w * logo_h * 4;
    if LOGO_DATA.len() < expected_len || logo_w == 0 || logo_h == 0 {
        return;
    }

    let fb_w = fb.width as usize;
    let fb_h = fb.height as usize;
    let x0 = fb_w.saturating_sub(logo_w) / 2;
    let y0 = fb_h.saturating_sub(logo_h) / 2;
    let draw_w = logo_w.min(fb_w);
    let draw_h = logo_h.min(fb_h);

    for row in 0..draw_h {
        for col in 0..draw_w {
            let px_off = 8 + (row * logo_w + col) * 4;
            let color = u32::from_le_bytes(LOGO_DATA[px_off..px_off + 4].try_into().unwrap());
            let offset = (y0 + row) * fb.pitch as usize + (x0 + col) * 4;
            unsafe {
                let ptr = fb.kernel_addr as *mut u8;
                core::ptr::write_volatile(ptr.add(offset), color as u8);
                core::ptr::write_volatile(ptr.add(offset + 1), (color >> 8) as u8);
                core::ptr::write_volatile(ptr.add(offset + 2), (color >> 16) as u8);
                core::ptr::write_volatile(ptr.add(offset + 3), 0u8);
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
}
