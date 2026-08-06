# S013698 implementation

Implemented `src/include/linux/device-id/auxiliary.rs` from pinned
`vendor/linux/include/linux/device-id/auxiliary.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The frozen scope marks this header `common` (the x86_64/AArch64 union). The
translation preserves `AUXILIARY_NAME_SIZE` (40), the `"auxiliary:"` module
prefix, and the `auxiliary_device_id` field order. `#[repr(C)]` keeps the C ABI
layout; `name` uses `c_char[40]` and `driver_data` uses `c_ulong`, matching the
header's `char[40]` and `unsigned long` on both frozen 64-bit targets.

No compilation, formatting, tests, or other runtime/build tooling was run.
