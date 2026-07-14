//! linux-parity: partial
//! linux-source: vendor/linux/drivers/gpu
//! GPU driver tree — M57+.
pub mod buddy;
pub mod drm;

pub fn register_module_exports() {
    buddy::register_module_exports();
}
