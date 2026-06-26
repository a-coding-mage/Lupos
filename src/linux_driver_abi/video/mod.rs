//! linux-parity: partial
//! linux-source: vendor/linux/drivers/video
//! Linux `drivers/video/` tree — currently just the legacy fbdev character
//! device used by the X.Org `fbdev` driver and Weston's fbdev backend.

pub mod console;
pub mod fbdev;
pub mod logo;
