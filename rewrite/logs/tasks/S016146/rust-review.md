# Rust source review — S016146 (slot 2)

Status: FINDINGS

Reviewed only the current candidate, its candidate diff, the complete pinned
`include/uapi/linux/hid.h`, the direct frozen task records, and the direct
`USB_TYPE_CLASS` definition in pinned `include/uapi/linux/usb/ch9.h`. No
compiler, formatter, test, runtime, rust-analyzer diagnostic, Git history, or
historical Lupos Rust source was used.

## Finding RUST-001 — blocking: C enum integer semantics were narrowed to Rust enum validity

Pinned Linux `enum hid_report_type` (lines 49–55) and `enum
hid_class_request` (lines 61–68) are C integer types. Their named enumerators
are unscoped integer constants; a C object of either enum type can carry an
integer representation other than one of the listed enumerators. The header
does not establish a closed set of valid values.

The candidate instead exports `#[repr(C)]` Rust enums at
`src/include/uapi/linux/hid.rs:25` and `:36`. Rust enum values have a
valid-discriminant invariant, so treating an arbitrary C integer as either
type would be invalid Rust state. It also changes the C names from unscoped
integer constants to associated variant names (`hid_report_type::...` and
`hid_class_request::...`). `repr(C)` does not repair either semantic change.
The frozen ABI records for both enum types remain `PENDING_REVIEW`, so the
candidate's selected Rust enum representation is additionally unsupported by
an accepted per-target ABI decision.

Resolve by preserving an integer representation that accepts all values the C
enum representation can carry, exposing the named C enumerators with their C
integer-constant semantics, and completing the exact x86_64/AArch64 ABI
decision from pinned source/configuration evidence. Do not use a Rust enum as
the replacement unless the resulting validity and namespace differences are
explicitly eliminated.

## Other Rust-semantics checks

The candidate has no `unsafe` blocks/functions, pointers, references,
borrows, aliases, pinning, interior mutability, `Send`/`Sync` assertions,
allocation, callbacks, drops, bounds checks, or panic/unwrap paths. Its
integer literal values and the inlined `USB_TYPE_CLASS` calculation match the
pinned macro value `(0x01 << 5)` for the reviewed targets; this does not cure
the enum-type finding above.
