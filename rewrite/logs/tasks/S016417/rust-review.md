# Rust source review — S016417 / attempt 1

Status: FINDINGS

Reviewed the complete pinned `include/uapi/linux/thermal.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the current candidate
`src/include/uapi/linux/thermal.rs`, the candidate snapshot, and the frozen
S016417 scope, symbol, ABI, and lifetime rows for both architectures. This was
manual source inspection only; no compiler, formatter, test, runtime tool, or
historical Rust source was used.

## RUST-001 — String macro representation changes the UAPI object and FFI contract

SC1 keys: `THERMAL_GENL_FAMILY_NAME`, `THERMAL_GENL_SAMPLING_GROUP_NAME`,
`THERMAL_GENL_EVENT_GROUP_NAME`

Pinned Linux lines 22, 24, and 25 define C string-literal macros. Each expands
to a `char[N]` literal including its terminating NUL and, in C expression
contexts, can decay to a pointer to that NUL-terminated storage. The candidate
exports `&str` values instead. A Rust `&str` is a UTF-8 slice with a
pointer-and-length representation, its exposed length excludes the C
terminator, and it neither has the C array type nor supplies the C pointer
decay behavior. Passing or embedding these values through a Linux-shaped FFI
or in a C-char-array initialization would therefore change both ABI and
observable bytes. Replace this representation with one that preserves the
literal bytes and terminating NUL and expose any pointer/array use only under
the same explicit contract as the C macro.

## RUST-002 — Fieldless Rust enums impose invalid-discriminant validity that C does not

SC1 keys: `thermal_device_mode`, `thermal_trip_type`, `thermal_genl_attr`,
`thermal_genl_sampling`, `thermal_genl_event`, `thermal_genl_cmd`

The six pinned C enums (lines 9–12, 14–19, 28–58, 61–64, 68–90, and 94–107)
are C integer-compatible enum types; their constants are integer values and
the Linux UAPI can carry an unrecognised/future integer through an enum-typed
object. The candidate makes each a Rust `#[repr(C)]` fieldless enum. That
introduces Rust discriminant validity: a value outside the listed variants
cannot soundly be materialized as that Rust type, including at an FFI or
netlink boundary, whereas the corresponding C storage remains an integer
object. `repr(C)` selects a C enum representation but does not remove Rust's
invalid-variant contract. Model these UAPI enum carriers with an integer
representation/aliases and named integer constants (or another representation
that explicitly admits every C integer bit pattern) before accepting the ABI.

## Checked without additional findings

The integer-literal macros use `core::ffi::c_int`, which matches these
non-suffixed C integer literals for the frozen x86_64 and AArch64 targets. The
derived `*_MAX` expressions retain their current numerical values and cannot
overflow for the listed sentinels. The candidate has no `unsafe`, allocation,
borrow, pinning, callback, refcount, interior-mutability, or Drop behavior to
audit. Its macro constants correctly remain Rust compile-time constants rather
than exported linker symbols; the findings above concern their represented
values and types.
