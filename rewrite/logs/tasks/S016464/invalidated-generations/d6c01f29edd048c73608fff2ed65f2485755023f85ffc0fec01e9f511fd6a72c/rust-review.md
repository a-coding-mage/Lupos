# Rust review: S016464

Reviewed `src/include/uapi/linux/virtio_ids.rs` against pinned
`include/uapi/linux/virtio_ids.h` for Rust type, ABI, and UAPI translation
semantics.

## Result

No findings.

The source header defines 47 object-like macros whose values are unsuffixed
integer literals.  On both approved Linux targets those literals have C `int`
type.  The candidate maps every macro to a public `core::ffi::c_int` constant;
`c_int` is the platform C `int` type and is `i32` for the approved x86_64 and
AArch64 targets.  This preserves the signed 32-bit literal category rather
than incorrectly narrowing the UAPI identifiers to `u16` merely because some
consumers later store them in 16-bit protocol fields.  Each translated caller
must retain the original C conversion at its own use site.

Name/value comparison found all 47 C macros present exactly once in Rust,
including the intentional normal-ID gaps `13 -> 16` and `41 -> 45`, and all
seven transitional values through `VIRTIO_TRANS_ID_9P = 0x1009`.  This
macro-only header defines no structs, FFI declarations, ownership, aliasing,
drop, synchronization, or unsafe operations.

No build, formatter, test, or runtime command was run.
