# Parity review — S013727

Reviewed only the pinned source `vendor/linux/include/linux/device-id/platform.h`
against `src/include/linux/device-id/platform.rs`, with the frozen Phase 0
records.  This was a manual source review; no compiler, formatter, test, or
runtime tool was invoked.

## Review basis

- Pinned revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`, matching the
  candidate provenance at `platform.rs:3`.
- Queue row: `S013727`, `REVIEWING`, pipeline `P01`, destination
  `src/include/linux/device-id/platform.rs`, Linux source
  `include/linux/device-id/platform.h`, architectures `common`.
- `rewrite/SCOPE.tsv` classifies this exact source as `RUST_TRANSLATE` for both
  frozen configurations.  `rewrite/metadata/header_closure.tsv` records 4,395
  AArch64 and 130 x86_64 consumers.  The selected-symbol records enumerate the
  guard, `__KERNEL__` conditional, both macros, `kernel_ulong_t`, and
  `struct platform_device_id` for both architectures.
- The recorded Kbuild commands for both architecture sets contain
  `-D__KERNEL__`, `-funsigned-char`, and their respective 64-bit targets;
  therefore the active typedef is `unsigned long` and `name` is an unsigned
  24-byte character array in the frozen scope.

## Finding P1 — `PLATFORM_MODULE_PREFIX` no longer has C string-literal macro semantics

The upstream object-like macro is exactly
`#define PLATFORM_MODULE_PREFIX "platform:"`
(`vendor/linux/include/linux/device-id/platform.h:10`).  It expands as a C
string-literal token and therefore supports literal concatenation and ordinary
C string-pointer decay at each use.  Pinned callers rely on both forms:

- `vendor/linux/drivers/gpu/drm/bridge/synopsys/dw-hdmi-cec.c:360` uses
  `MODULE_ALIAS(PLATFORM_MODULE_PREFIX "dw-hdmi-cec")`.
- `vendor/linux/scripts/mod/file2alias.c:962` uses it as the first portion of a
  format string: `PLATFORM_MODULE_PREFIX "%s"`.
- `vendor/linux/drivers/base/platform.c:1409` passes it as a `%s` argument.

The candidate instead declares an exported, zero-argument Rust macro at
`platform.rs:27-32` whose expansion is `b"platform:\\0"`.  That is a byte-slice
literal expression with an explicit terminal byte, not a string-literal token.
Consequently it cannot participate in the upstream literal-concatenation forms
(for example, `PLATFORM_MODULE_PREFIX!() b"%s"` is not one literal), and it
does not have C's implicit decay to a `char *` formatting argument.  The
candidate documentation at lines 23-26 does not restore those missing usage
semantics.

Required resolution: represent this macro through the project’s macro/string
interoperation mechanism so every selected Rust-side translation can preserve
the source forms above (including literal concatenation and the one, final C
NUL terminator at FFI boundaries), or establish and record the exact
source-level mechanism that preserves those forms.  Do not accept a byte slice
as a replacement string token without that proof.

## Checked without finding

- `kernel_ulong_t`: upstream lines 5-7 conditionally define `unsigned long`;
  candidate lines 7-10 use `core::ffi::c_ulong`.  With the frozen kernel-only
  LP64 command targets, this retains the required width and unsignedness.
- `PLATFORM_NAME_SIZE`: upstream line 9 is the unsuffixed integer literal 24;
  candidate lines 12-21 retains an `i32` literal and explicitly converts only
  at the Rust array-bound use, matching C's `int` macro type in the selected
  architectures.
- `struct platform_device_id`: upstream lines 12-15 are `char name[24]` then
  `kernel_ulong_t driver_data`.  Candidate lines 34-42 preserve field order,
  24-byte unsigned-character storage under the recorded `-funsigned-char`,
  and `#[repr(C)]`.  On both frozen LP64 targets this yields the upstream
  24-byte first field followed by an 8-byte naturally aligned unsigned-long
  field (32-byte aggregate, 8-byte alignment).
- No functions, statics, selected configuration branches, branding deltas,
  placeholders, or Rust test configuration occur in the upstream header or
  candidate beyond the issue above.  The provenance lines 1-5 carry the
  required SPDX/source/revision/architectures/task identity.

## Verdict

Reject pending resolution of P1.  The remaining selected type, layout, and
integer macro mappings are source-equivalent for the frozen configurations.
