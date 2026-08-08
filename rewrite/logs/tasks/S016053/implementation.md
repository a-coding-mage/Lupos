# S016053 implementation

Implemented the complete selected `include/uapi/linux/arm_sdei.h` header as
`src/include/uapi/linux/arm_sdei.rs` for AArch64.

Source evidence: pinned Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`,
complete source file, `include/linux/arm_sdei.h` inclusion context,
`arch/arm64/kernel/sdei.c` callers, and the `CONFIG_ARM_SDE_INTERFACE` Kconfig
and Kbuild entries were inspected. The header contains only constants and
expression macros; no structs, FFI declarations, locking, or lifetime-bearing
objects are selected.

The Rust file preserves all function identifiers, function-number arithmetic,
version extraction shifts and masks, return values, event flags/status values,
event-info selectors, event types, and priorities. Expression macros remain
macro-based so their argument is evaluated at the call site as in C.
