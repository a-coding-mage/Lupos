# Implementation — S013711

Translated `include/linux/device-id/i2c.h` to `src/include/linux/device-id/i2c.rs` from pinned Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.

The selected x86_64 and AArch64 configurations both set `CONFIG_I2C=y`; their in-kernel header view enables the source's `__KERNEL__` typedef. `kernel_ulong_t` is therefore the native unsigned long width, `u64` on both selected LP64 architectures. The `#[repr(C)]` structure preserves the 20-byte name array followed by native-word `driver_data` (including C ABI alignment and tail padding). The module-prefix macro is represented as its exact NUL-terminated C-string byte literal.

No compiler, formatter, linker, test, or runtime command was run.
