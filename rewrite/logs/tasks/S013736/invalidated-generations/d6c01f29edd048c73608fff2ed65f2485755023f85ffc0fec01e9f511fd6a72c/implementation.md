# Implementation — S013736

Translated `vendor/linux/include/linux/device-id/spmi.h` at pinned revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` to
`src/include/linux/device-id/spmi.rs` for the frozen x86_64 and AArch64 union.

The translation preserves the `__KERNEL__` `kernel_ulong_t` alias as C
`unsigned long`, the `int`-typed `SPMI_NAME_SIZE` macro, the NUL-terminated
`SPMI_MODULE_PREFIX` literal, and the two-field `#[repr(C)]` ID-table layout.
The `name` field remains a fixed-width unsigned-char byte array; `driver_data`
remains opaque machine-word driver data.

Source context checked: the Phase 0 scope/map/symbol/ABI/lifetime records,
both frozen configurations, header-closure selection records, and the pinned
SPMI core's `SPMI_NAME_SIZE` use in `drivers/spmi/spmi.c`.
