# Rust review — S013711

Scope reviewed: `src/include/linux/device-id/i2c.rs` against
`vendor/linux/include/linux/device-id/i2c.h`, the S013711 Phase-0 records, and
the frozen x86_64/AArch64 command metadata. This was a manual source review;
no compiler, formatter, linker, or test tool was invoked.

## Result: changes required

### R1 — `i2c_device_id` lost C's by-value copy semantics (high)

The C record at `include/linux/device-id/i2c.h:14-17` is a plain aggregate, so
assignment, argument/return passing by value, and aggregate initialization copy
its complete object representation. The candidate's `#[repr(C)]` record at
`src/include/linux/device-id/i2c.rs:13-16` does not implement `Copy` or
`Clone`; therefore any translated caller cannot perform the corresponding
ordinary Rust value copy without moving the source value. This is a semantic
change even though the physical layout is otherwise correct.

Required resolution: make this simple two-field POD record explicitly
`#[derive(Copy, Clone)]`. Do not introduce ownership, drop behavior, or a
non-C backing allocation.

### R2 — `I2C_NAME_SIZE` has the wrong C expression type (medium)

Upstream's `#define I2C_NAME_SIZE 20` at line 11 is an unsuffixed C integer
literal and consequently has `int` type in expressions. The candidate exports
it as `usize` at line 9. That changes Rust callers' inferred arithmetic,
comparisons, casts, and overflow domain from the selected C `int` semantics.

Required resolution: represent the macro as an `i32` literal (or an
equivalent macro expanding to `20i32`) and use an explicit `as usize` only at
the Rust array-bound site.

### R3 — `I2C_MODULE_PREFIX` is lowered to a named reference rather than a
string-literal macro (medium)

The C macro at `i2c.h:12` expands at every use to the C string-literal token
`"i2c:"`: a NUL-terminated unsigned-character array under the frozen
`-funsigned-char` commands. In particular, the source uses it in adjacent
literal expressions at `drivers/media/v4l2-core/v4l2-i2c.c:74` and
`scripts/mod/file2alias.c:860` (`I2C_MODULE_PREFIX "%s"`). The candidate's
`pub const I2C_MODULE_PREFIX: &[u8; 5]` at line 10 instead declares a Rust
reference value. It cannot preserve those literal-token/concatenation uses and
changes the macro's expression type and object/address semantics.

Required resolution: provide a macro-level representation that expands to the
NUL-terminated byte literal for ordinary uses, plus the narrowly scoped
literal-concatenation lowering needed by translated call sites (following the
project's existing C-string macro pattern). Preserve the five bytes
`i2c:\\0`; do not substitute a Rust `str` or an owned allocation.

### R4 — required provenance SPDX identifier differs from the Phase-1 form
(medium)

The candidate begins with `// SPDX-License-Identifier: GPL-2.0` at line 1.
The mandatory fresh-source provenance form requires
`GPL-2.0-only`. Update only this immutable provenance spelling; no license or
branding interpretation is needed.

## Verified items

- `kernel_ulong_t = u64` matches the `unsigned long` typedef selected by the
  `__KERNEL__` branch on both frozen 64-bit LP64 targets.
- Both frozen command families recorded under
  `rewrite/compiler-predicates/commands/` include `-funsigned-char`; thus the
  source `char name[20]` is correctly represented with unsigned octet storage,
  not signed `c_char`.
- `#[repr(C)]`, field order, the 20-byte array, and 64-bit `driver_data`
  produce the required 8-byte alignment, `driver_data` offset 24 (four bytes
  of C padding), and 32-byte total record size on both selected targets.
- This header contains no callback, pointer provenance, aliasing, pinning,
  interior-mutability, `unsafe`, `Drop`, or `Send`/`Sync` mechanism requiring
  an additional Rust finding.
