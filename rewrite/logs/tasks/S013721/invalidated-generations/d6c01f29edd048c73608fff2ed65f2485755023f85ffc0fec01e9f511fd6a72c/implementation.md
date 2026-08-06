# Implementation — S013721

Implemented `src/include/linux/device-id/mhi.rs` from the complete pinned oracle `vendor/linux/include/linux/device-id/mhi.h` at `425f94c2954b1fe80ebdbf9b29854e89750355df`.

The common x86_64/AArch64 header has the include guard, its kernel-only `kernel_ulong_t` typedef, two modalias string-literal macros, `MHI_NAME_SIZE`, and `struct mhi_device_id`.  The Rust translation retains the target-width unsigned-long ABI as `u64` for both frozen 64-bit targets; retains modalias strings as static NUL-terminated C-literal storage with thin pointer views; and gives the device-ID record `#[repr(C)]`, a 32-octet channel field, and the original field order.  The frozen compile commands use `-funsigned-char`, hence `chan` is represented by `[u8; 32]` rather than signed `c_char`.

Relevant local context was inspected in `include/linux/mhi.h`, `include/linux/mhi_ep.h`, host/endpoint MHI matching paths, and representative selected driver ID-table initializers.  Those contexts retain pointers to this record and use the static channel array and opaque driver word exactly as declared.  There is no dynamic ownership, locking, allocation, control flow, or error path in the source header.

Queue verification, the P01 lease (attempt 1), branch, frozen Linux SHA, and Phase 0 identity were checked before editing.  No historical Lupos source, compiler, formatter, test, runtime command, or non-leased source file was used or changed.
