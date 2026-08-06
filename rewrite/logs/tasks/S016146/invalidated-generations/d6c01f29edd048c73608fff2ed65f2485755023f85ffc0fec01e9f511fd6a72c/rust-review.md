# Rust review — S016146

Reviewed `src/include/uapi/linux/hid.rs` against pinned
`vendor/linux/include/uapi/linux/hid.h` for the common x86_64/AArch64 scope.
No compiler, formatter, test, or runtime command was run.

## Rust ABI and semantic review

- **C enums:** accepted.  Both selected consumer compile commands use the
  pinned Clang target (`x86_64-linux-gnu` and `aarch64-linux-gnu`) and contain
  no `-fshort-enums` option.  The two C enum tags therefore have the normal
  C `int` ABI for this frozen input.  `core::ffi::c_int` is the appropriate
  target-C-`int` scalar on both targets; the aliases add no layout, drop, or
  ownership behavior.
- **Enumerators:** accepted.  `hid_report_type` retains `0`, `1`, `2`, and
  the sentinel `HID_REPORT_TYPES == 3`; `hid_class_request` retains `0x01`,
  `0x02`, `0x03`, `0x09`, `0x0a`, and `0x0b`.  These are all C integer
  constant expressions and the candidate exposes the same values as C-int
  constants.
- **Macros:** accepted.  The exact source definition of `USB_TYPE_CLASS` in
  `include/uapi/linux/usb/ch9.h:55` is `(0x01 << 5)`.  Consequently the Rust
  expressions yield the same C-int values for `HID_DT_HID` (`0x21`),
  `HID_DT_REPORT` (`0x22`), and `HID_DT_PHYSICAL` (`0x23`); the remaining
  numeric macros also agree exactly.  This header declares no string-literal
  macro, so no C string storage/decay contract is missing.
- **Configuration and Rust hazards:** accepted.  The source has no selected
  Kconfig conditional branch, data layout, pointer, FFI export, allocation,
  unsafe block, or ownership/lifetime behavior.  The `common` task scope is
  preserved.

## Required non-Rust source correction

**R1 — upstream copyright notices omitted (required).**  The candidate keeps
the SPDX expression but omits the three upstream copyright notices from
`include/uapi/linux/hid.h:2-6`.  The rewrite protocol requires relevant
upstream copyright notices to be retained.  The applier must add those notices
to the destination without changing the reviewed definitions.

Other than R1, this review has no Rust semantic, ABI, layout, unsafe, or
macro-value finding.
