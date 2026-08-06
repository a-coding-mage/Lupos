# Parity review — S016454

Reviewed source-only against `vendor/linux/include/uapi/linux/vesa.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the frozen task records, header
closure evidence, and the branding allowlist.  The task row is `REVIEWING` in
pipeline `P02`; its frozen source and destination are respectively
`include/uapi/linux/vesa.h` and `src/include/uapi/linux/vesa.rs` for `common`
(x86_64 and aarch64 selection evidence).  No compiler, formatter, test,
debugger, or historical Lupos source was used.

## Findings

1. **Missing exported `VESA_BLANK_MAX` enumerator (blocking parity defect).**
   Linux declares `VESA_BLANK_MAX = VESA_POWERDOWN` in
   `vendor/linux/include/uapi/linux/vesa.h:15`; as an enum enumerator it is an
   unqualified identifier available to all includers, with integer value `3`.
   The Rust candidate defines only the associated item
   `vesa_blank_mode::VESA_BLANK_MAX`; it does not define the corresponding
   public module-level `VESA_BLANK_MAX: i32`, unlike its module-level mappings
   for the preceding four enumerators.  Consequently downstream translations
   cannot use the Linux spelling/value in the same integer expressions.  The
   direct selected-source inventory omission in `SYMBOLS.tsv` does not override
   the pinned header, which is the implementation oracle.

2. **The Rust enum-value wrapper is not copyable, unlike the C enum value
   type (parity defect).**  `enum vesa_blank_mode` at
   `vendor/linux/include/uapi/linux/vesa.h:6-16` is a C scalar enum: values may
   be copied by ordinary assignment and passed by value.  The candidate's
   `#[repr(transparent)] pub struct vesa_blank_mode(pub i32)` has neither
   `Copy` nor `Clone`; therefore assigning or passing a previously held Rust
   value consumes it.  This changes the value semantics of the selected type
   and prevents direct translations of its ordinary C value use without an
   unrelated workaround.  The replacement must retain the C scalar-copy
   behavior while preserving the chosen ABI representation.

3. **SPDX identifier is not retained (provenance/licensing parity defect).**
   The sole Linux header identifier at `vendor/linux/include/uapi/linux/vesa.h:1`
   is `GPL-2.0 WITH Linux-syscall-note`.  The candidate begins with
   `GPL-2.0-only`, which is a different SPDX expression.  This is neither an
   allowlisted branding delta (the frozen allowlist has no VESA entry) nor the
   source identifier that the rewrite rules require to be retained.

## Confirmed mappings

`VESA_NO_BLANKING`, `VESA_VSYNC_SUSPEND`, `VESA_HSYNC_SUSPEND`, and
`VESA_POWERDOWN` have the correct integer values `0`, `1`, `2`, and `1 | 2`
(`3`) in the candidate's public module-level constants.  The header has no
configuration conditional around these declarations.  Header-closure evidence
selects it for both architectures (`rewrite/metadata/header_closure.tsv`,
rows for aarch64 and x86_64); `rewrite/SCOPE.tsv` maps it as a common
`RUST_TRANSLATE` file.  No other selected declarations occur in the 18-line
oracle header.

## Result

Parity review is complete with the three findings above.  No source changes
were made by this reviewer.
