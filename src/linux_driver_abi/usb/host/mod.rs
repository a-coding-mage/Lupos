//! linux-parity: partial
//! linux-source: vendor/linux/drivers/usb/host
//! USB host controller drivers — M58.
pub mod xhci;
pub use xhci::XhciHcd;
