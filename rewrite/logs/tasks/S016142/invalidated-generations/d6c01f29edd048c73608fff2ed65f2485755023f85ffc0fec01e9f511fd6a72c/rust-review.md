# Rust review — S016142 (slot 2)

Verdict: **REJECT**.

Reviewed the complete pinned `include/uapi/linux/handshake.h` against
`src/include/uapi/linux/handshake.rs` at Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` for the common x86_64/aarch64
scope. This was a source-only review; no build, formatter, test, or runtime
command was run.

## Finding

### RUST-1 — high — named-enum enumerators no longer have C `int` expression semantics

The three tagged C enums at `handshake.h:13-29` declare enum tags, but their
enumerator identifiers are C enumeration constants with type `int` under the
frozen C language mode. Thus, for example,
`HANDSHAKE_HANDLER_CLASS_TLSHD` can be passed, assigned, and used directly as
an `int` expression; it is not an instance of a distinct wrapper object.

The candidate instead gives every named enumerator a distinct
`#[repr(transparent)]` newtype at Rust lines 23-41. A translated consumer that
needs the integer value (including the YAML-defined `u32` generic-netlink
attribute payloads for handler-class, message-type, and auth-mode) must now
extract `.0` or introduce an ad-hoc conversion. This changes expression and
assignment behavior despite preserving the four-byte representation.

Resolve this together with the still-`PENDING_REVIEW` ABI records for the three
enum tags. If the frozen ABI establishes the candidate's `c_int` storage
choice, expose each tag as a `c_int` alias and its enumerators as `c_int`
constants, which retains the C enumeration-constant use sites without an
invalid-value Rust enum or wrapper conversion. Do not substitute `u32` merely
because the netlink wire attributes are `u32`; the source enum's C ABI must be
established independently.

## Checks with no discrepancy

- The four anonymous C enums are faithfully represented as `c_int` constants:
  all explicit starts, implicit increments, private `__*_MAX` sentinels, and
  derived public `*_MAX` expressions have the source values.
- `HANDSHAKE_FAMILY_VERSION` is the source integer value `1` as `c_int`.
- The three C string-literal macros are exact ASCII, NUL-terminated immutable
  `[c_char; N]` arrays (`handshake\0`, `none\0`, and `tlshd\0`). They preserve
  static backing storage and permit explicit C array-to-pointer decay with
  `.as_ptr()` without creating a Rust `&str` or a fat pointer.
- There are no C structures, unions, pointer fields, functions, selected
  Kconfig branches, unsafe blocks, allocations, `Drop` implementations, or
  Rust test configuration in this task. `repr(transparent)` itself gives the
  wrapper its single-field ABI; it is the changed enumerator expression type,
  not an unsafe or layout defect, that requires resolution.
- SPDX and immutable provenance identify the exact source path, revision,
  common architecture scope, and task ID.
