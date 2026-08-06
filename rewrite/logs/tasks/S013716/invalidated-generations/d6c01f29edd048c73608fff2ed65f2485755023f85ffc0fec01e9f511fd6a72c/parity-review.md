# Parity review — S013716 (slot 1)

## Scope and evidence

- Queue row `S013716` was `REVIEWING` on pipeline `P01`; its frozen source and
  destination are `include/linux/device-id/isapnp.h` and
  `src/include/linux/device-id/isapnp.rs`, respectively.
- `vendor/linux.SHA`, the checked-out `vendor/linux` revision, and candidate
  provenance all identify `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- `rewrite/SCOPE.tsv:13717` classifies the file as common `RUST_TRANSLATE`.
  The frozen header-closure evidence selects it for both targets
  (`rewrite/metadata/header_closure.tsv:4564,9563`), and the recorded original
  compile commands define `__KERNEL__` for both targets
  (`rewrite/FILE_MAP.tsv:16494,21493`).

## Exhaustive comparison

- `kernel_ulong_t`: Linux declares `typedef unsigned long kernel_ulong_t` in
  the selected `__KERNEL__` branch at
  `vendor/linux/include/linux/device-id/isapnp.h:5-7`; the candidate maps it
  to `core::ffi::c_ulong` at `src/include/linux/device-id/isapnp.rs:7-8`.
  The frozen x86_64 and AArch64 source definitions establish 64-bit longs
  (`vendor/linux/arch/x86/include/uapi/asm/bitsperlong.h:5-8` and
  `vendor/linux/arch/arm64/include/uapi/asm/bitsperlong.h:20-25`).
- `ISAPNP_ANY_ID`: Linux defines the sole operative macro as the unsuffixed
  literal `0xffff` at `isapnp.h:9`.  The candidate preserves that value and
  its frozen-target C `int` type as `core::ffi::c_int` at `isapnp.rs:10-11`.
- `struct isapnp_device_id`: Linux contains exactly four ordered `unsigned
  short` members followed by `kernel_ulong_t driver_data` at `isapnp.h:10-14`.
  The candidate retains every member in that order as four `u16` fields and
  `kernel_ulong_t`, with `#[repr(C)]` at `isapnp.rs:14-23`.  Thus it preserves
  the C field widths, order, natural alignment/padding, and both-target ABI.
  `Copy, Clone` adds no state or layout and is consistent with C structure
  value copying.
- The source has no functions, statics, arrays, string literals, Kconfig
  branches, or additional operative macros.  Its only conditionals are the
  include guard and the selected `__KERNEL__` typedef branch
  (`isapnp.h:2-7,16`); neither represents a missing runtime behavior in the
  frozen kernel-only translation context.
- Candidate SPDX, Linux source path, revision, common architecture membership,
  and task ID at `isapnp.rs:1-5` agree with the pinned source/task identity.
  There is no branding delta.

## Result

No parity findings.  The candidate covers every selected declaration, literal,
field, layout-relevant type, conditional, and provenance item in the pinned
header for x86_64 and AArch64.  No source was modified and no compiler,
formatter, analyzer, build, or test was run.
