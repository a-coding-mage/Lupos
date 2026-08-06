# Resolution — S016146

Reviewed and resolved against pinned `vendor/linux/include/uapi/linux/hid.h`
at `425f94c2954b1fe80ebdbf9b29854e89750355df`.  No compiler, formatter, test,
linker, emulator, or runtime command was run.

## Parity finding P1 — distinct enum tags

**Resolved.**  `hid_report_type` and `hid_class_request` are now separate
`#[repr(transparent)]` tuple structs over `core::ffi::c_int`, rather than type
aliases.  They preserve the distinct C tags used by
`include/linux/hid.h:568,1031,1039,1043,1215,1219` while preserving the scalar
C ABI.  A transparent scalar wrapper accepts every `c_int` bit pattern, so it
does not impose Rust fieldless-enum valid-discriminant restrictions on values
received at an ABI boundary.

The frozen HID consumer commands in
`rewrite/metadata/{x86_64,aarch64}/compile_commands.json` use the pinned LLVM
19 clang with `--target=x86_64-linux-gnu` and `--target=aarch64-linux-gnu`,
respectively, both under `-std=gnu11` and without `-fshort-enums`.  The source
enumerators at `include/uapi/linux/hid.h:49-68` fit signed C `int`; the
task-specific ABI records therefore establish the four-byte, alignment-four
C-int enum ABI for both targets.  The matching lifetime records establish that
these declarations introduce no object, storage, ownership, or locking state.

## Parity finding P1 / Rust finding R1 — copyright notices

**Resolved.**  Restored the three upstream notices for Andreas Gal, Vojtech
Pavlik, and Jiri Kosina immediately after the immutable provenance block.  The
SPDX expression and all provenance lines remain unchanged.

## Rust review acceptance

Accepted after the above source correction.  The named constants retain their
pinned source values, including the chapter-9 `USB_TYPE_CLASS` expression
from `include/uapi/linux/usb/ch9.h:55`; this gives HID descriptor values
`0x21`, `0x22`, and `0x23`.  The header contains no functions, mutable state,
allocation, ownership transfer, locking, or unsafe code.
