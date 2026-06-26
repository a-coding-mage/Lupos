//! linux-parity: complete
//! linux-source: vendor/linux/lib/kunit
//! KUnit built-in support objects.

pub mod hooks;
pub mod try_catch;

pub fn register_module_exports() {
    hooks::register_module_exports();
    try_catch::register_module_exports();
}
