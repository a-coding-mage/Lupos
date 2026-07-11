//! linux-parity: partial
//! linux-source: vendor/linux/drivers/video/nomodeset.c
//! `nomodeset` state exported to native graphics drivers.

use core::sync::atomic::{AtomicBool, Ordering};

static VIDEO_NOMODESET: AtomicBool = AtomicBool::new(false);

pub fn configure_from_cmdline(cmdline: &str) {
    let disabled = cmdline
        .split_ascii_whitespace()
        .any(|token| token == "nomodeset" || token.starts_with("nomodeset="));
    VIDEO_NOMODESET.store(disabled, Ordering::Release);
    if disabled {
        crate::log_warn!(
            "",
            "Booted with the nomodeset parameter. Only the system framebuffer will be available"
        );
    }
}

/// `video_firmware_drivers_only()` from `drivers/video/nomodeset.c`.
#[unsafe(no_mangle)]
pub extern "C" fn video_firmware_drivers_only() -> bool {
    VIDEO_NOMODESET.load(Ordering::Acquire)
}
