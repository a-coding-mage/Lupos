# Rust review — S013730

Scope reviewed manually: `src/include/linux/device-id/rpmsg.rs` against
`vendor/linux/include/linux/device-id/rpmsg.h`, the frozen x86_64/AArch64
configuration records, and direct selected-source consumers.  No compiler,
formatter, linker, test, or Rust-analyzer diagnostic was invoked.

## Result: changes required

### R1 — `rpmsg_device_id` does not retain C aggregate copy semantics (high)

`struct rpmsg_device_id` in the pinned header (lines 14–17) is a scalar-only
C aggregate.  A C assignment, aggregate initialization, or by-value argument
copies its complete representation and never transfers ownership.  The Rust
`#[repr(C)]` structure at lines 12–15 has no `Copy` or `Clone` implementation,
so an equivalent Rust expression moves the source and makes subsequent use
invalid.  Its eventual use in static ID tables, including the selected
`net/qrtr/smd.c:92–95`, must preserve this ordinary POD value behavior.

Required resolution: derive `Copy, Clone` directly on this two-field record.
Do not add ownership, a destructor, or an allocation.

### R2 — `RPMSG_NAME_SIZE` was changed from an `int` macro expression to a
`usize` constant (medium)

The upstream object-like macro at `rpmsg.h:11` expands to the unsuffixed C
integer literal `32`, whose C expression type is `int`.  The candidate exposes
`pub const RPMSG_NAME_SIZE: usize = 32`; this changes inferred arithmetic,
comparison, conversion, and overflow semantics at each translated use.  The
array bound is not justification for changing the macro's type: only that
specific Rust bound needs an explicit conversion.

Required resolution: represent the macro as an `i32`-typed expansion/value
(for example `32i32`) and use `as usize` solely for the `[u8; ...]` bound.

### R3 — the modalias macro has become a reference value and cannot preserve
C string-literal use or concatenation (medium)

At `rpmsg.h:12`, `RPMSG_DEVICE_MODALIAS_FMT` expands to the C string-literal
token `"rpmsg:%s"`, including its terminating byte when materialized.  The
candidate instead defines an addressable `&[u8; 9]` constant.  That is a
different expression/object category and cannot reproduce literal adjacency.
The original makes such use directly in `drivers/rpmsg/rpmsg_core.c:371`
(`RPMSG_DEVICE_MODALIAS_FMT "\\n"`) and in `:424`
(`"MODALIAS=" RPMSG_DEVICE_MODALIAS_FMT`).

Required resolution: retain a macro-level lowering which expands to the exact
NUL-terminated byte literal `b"rpmsg:%s\\0"`, and preserve/lower the direct
literal-concatenation uses without replacing the macro with a Rust `str`,
named reference, or owned buffer.

## Verified items

- `kernel_ulong_t = u64` correctly represents the `__KERNEL__`-selected
  `unsigned long` typedef for both frozen 64-bit configurations.
- The fixed `char[32]` field is correctly represented as 32 octets for the
  recorded unsigned-char target commands; `u8` also preserves the field's raw
  byte representation for FFI.
- `#[repr(C)]`, field order, and the scalar member types yield the required
  layout on both targets: 32-byte name at offset 0, 8-byte `driver_data` at
  offset 32, alignment 8, and total size 40.  There is no implicit padding
  between these fields.
- This declaration has no pointer, callback, `unsafe`, aliasing, pinning,
  interior-mutability, `Drop`, or concurrency mechanism needing a separate
  lifetime/provenance finding.
