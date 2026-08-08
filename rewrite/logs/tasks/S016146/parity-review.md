# Parity review — S016146 / P02 / slot 1

Scope: manual source-only comparison of pinned `vendor/linux/include/uapi/linux/hid.h`, current `src/include/uapi/linux/hid.rs`, the current candidate diff, frozen `SYMBOLS.tsv`/`ABI.tsv`, the allowlist, and direct pinned HID/USB header context.  No compiler, formatter, test, or runtime tool was invoked.

## Findings

1. **P1 — Linux symbols `enum hid_report_type`, `enum hid_class_request`, and all of their enumerators: the Rust enums do not preserve the C enumerators' integer-constant interface or full C enum value domain.**

   Local evidence: the pinned header declares `enum hid_report_type` at `include/uapi/linux/hid.h:49-55` and `enum hid_class_request` at `:61-68`.  Its enumerators (`HID_INPUT_REPORT` through `HID_REPORT_TYPES`, and `HID_REQ_GET_REPORT` through `HID_REQ_SET_PROTOCOL`) are unqualified C integer constants.  Direct consumers use that interface as an integer/ABI boundary: `include/linux/hid.h:690` declares `report_enum[HID_REPORT_TYPES]`, `:1031-1043` exports prototypes carrying both enum types, and `:1215-1226` continues those ABI-carrying requests.  The frozen inventory selects every enumerator and both enum types for x86_64 and aarch64; the corresponding `ABI.tsv` records remain `PENDING_REVIEW`.

   Candidate evidence: `src/include/uapi/linux/hid.rs:28-47` instead declares two Rust `#[repr(C)]` enums.  Their names are scoped (`hid_report_type::HID_INPUT_REPORT`, etc.), their variants have the enum type rather than C's unqualified integer-constant interface, and values outside the listed discriminants cannot be represented as valid Rust enum values even though the C enum object/pointer ABI can carry its compatible integer representation.  This changes both source interface and the set of representable ABI inputs for the HID-core function-pointer/prototype paths.  Replace these with a representation that establishes the pinned C-compatible integer layout for both architectures, preserves the valid raw integer domain, and exports the selected enumerator names with their C integer-constant semantics; close the two associated ABI records rather than relying on an unverified `repr(C)` assumption.

2. **P1 — Linux symbol `_UAPI__HID_H`: the selected include-guard macro is omitted.**

   Local evidence: `include/uapi/linux/hid.h:26-27` tests and defines `_UAPI__HID_H`, and closes it at `:81`.  `SYMBOLS.tsv` selects both the `ifndef@26` conditional and operative macro `_UAPI__HID_H` for each frozen architecture.  `src/include/uapi/linux/hid.rs:1-58` contains no corresponding guard representation or documented UAPI-header emission/compatibility mechanism.  Rust module loading alone is not source evidence that the selected UAPI preprocessor symbol and repeated-include behavior are preserved for the Linux-facing interface.  Implement or explicitly establish the frozen mechanism that preserves this selected conditional/macro contract.

3. **P2 — Linux symbols `HID_DT_HID`, `HID_DT_REPORT`, and `HID_DT_PHYSICAL`: the candidate substitutes a private literal calculation for the source macro dependency on `USB_TYPE_CLASS`.**

   Local evidence: the pinned definitions at `include/uapi/linux/hid.h:74-76` are macro expressions containing the separately defined UAPI symbol `USB_TYPE_CLASS`; direct local source defines that symbol as `(0x01 << 5)` in `include/uapi/linux/usb/ch9.h:53-57`.  The three HID macros are all selected operative macros in `SYMBOLS.tsv` for both architectures.  Candidate `hid.rs:52-56` merely comments on the dependency and hard-codes `((0x01_i32) << 5)` in three independent constants, with no import, alias, or preserved macro-level relationship to the ch9 definition.  The present numerical values are 0x21, 0x22, and 0x23, but the macro mechanism and its shared source-level dependency are lost.  Preserve the exact frozen dependency/constant mechanism rather than duplicating its current expansion.

## Checked without a finding

- `USB_INTERFACE_CLASS_HID`, `USB_INTERFACE_SUBCLASS_BOOT`, `USB_INTERFACE_PROTOCOL_KEYBOARD`, `USB_INTERFACE_PROTOCOL_MOUSE`, and `HID_MAX_DESCRIPTOR_SIZE` retain their pinned integer values in `hid.rs:16-23,58`; `HID_MAX_DESCRIPTOR_SIZE` remains 4096, matching the direct `hidraw_report_descriptor.value[HID_MAX_DESCRIPTOR_SIZE]` use at `include/uapi/linux/hidraw.h:22-25`.
- The candidate retains the pinned SPDX expression, all three upstream copyright notices, immutable provenance fields, original Linux identifiers, and no branding delta.  `rewrite/BRANDING_ALLOWLIST.tsv` contains only its header, so no branding difference is allowlisted or present.
- No function, allocation, locking, RCU, refcount, cleanup, error, or ordering path exists in this header beyond the interface/ABI effects identified above.

## Slot-1 attestation

Review completed manually against the current candidate and direct frozen source context.  Findings remain open; this report makes no compile, link, test, formatter, or runtime claim.
