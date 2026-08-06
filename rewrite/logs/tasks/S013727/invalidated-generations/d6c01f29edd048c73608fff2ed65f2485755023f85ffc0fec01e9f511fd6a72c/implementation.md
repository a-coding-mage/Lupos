# S013727 implementation

Implemented `src/include/linux/device-id/platform.rs` from the complete pinned
`include/linux/device-id/platform.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` for the frozen x86_64 and AArch64
configurations.

The translation preserves `kernel_ulong_t` as the frozen 64-bit `unsigned
long`, the 24-byte `name` member, C field order/native layout through
`#[repr(C)]`, and the NUL terminator of `PLATFORM_MODULE_PREFIX`.
