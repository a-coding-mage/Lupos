# Parity review — S016146

Reviewed `src/include/uapi/linux/hid.rs` against the complete pinned
`vendor/linux/include/uapi/linux/hid.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, together with the frozen x86_64
and AArch64 header-consumer commands and `include/uapi/linux/usb/ch9.h`.
No build, formatter, test, or runtime command was run.

## Findings

1. **P1 — distinct C enum types are collapsed to aliases.**
   Upstream declares two distinct tagged types, `enum hid_report_type` (lines
   49–57) and `enum hid_class_request` (lines 61–68), and uses them as
   separate function-interface types (for example
   `drivers/hid/hid-core.c:88`, `:2001`, and `:2518`).  The candidate declares
   `pub type hid_report_type = c_int` and
   `pub type hid_class_request = c_int`; aliases erase the tag distinction,
   permit values of either category to be interchanged without a conversion,
   and assert `c_int` as the ABI representation without a completed frozen
   ABI record.  Resolve this with two distinct ABI-preserving Rust types and
   establish their exact frozen C enum representation before closing the
   corresponding ABI records.  Do not use a Rust fieldless enum if its valid
   discriminant restriction would reject C-provided out-of-range values.

2. **P1 — required upstream copyright notices were not retained.**
   The candidate preserves the SPDX expression but omits the upstream notices
   for Andreas Gal, Vojtech Pavlik, and Jiri Kosina in the source header
   (lines 2–6).  The project source-tree rule requires retaining relevant
   upstream copyright notices.  Restore those notices in the destination
   without changing the immutable provenance lines.

## Verified coverage

- All eight named macros are present with their upstream values:
  `USB_INTERFACE_CLASS_HID`, `USB_INTERFACE_SUBCLASS_BOOT`,
  `USB_INTERFACE_PROTOCOL_KEYBOARD`, `USB_INTERFACE_PROTOCOL_MOUSE`,
  `HID_DT_HID`, `HID_DT_REPORT`, `HID_DT_PHYSICAL`, and
  `HID_MAX_DESCRIPTOR_SIZE`.
- Both enum tag names and all ten enumerators are represented with their
  upstream numeric values.  `HID_REPORT_TYPES` remains `3` after the three
  zero-based report kinds.
- `USB_TYPE_CLASS` is `(0x01 << 5)` in
  `include/uapi/linux/usb/ch9.h:55`; consequently the three descriptor
  expressions evaluate to `0x21`, `0x22`, and `0x23` as represented.
- The include guard has no runtime/layout analogue in the path-preserving Rust
  module.  This header has no structs, functions, mutable data, ownership,
  locking, or runtime state.
- SPDX and all immutable Linux source/revision/architecture/task provenance
  lines match the task and pinned revision.

**Result: changes required before parity acceptance.**
