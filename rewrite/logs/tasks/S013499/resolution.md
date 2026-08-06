# S013499 applier resolution

Reviewer findings were resolved from the complete pinned source
`vendor/linux/include/linux/bcma/bcma_driver_arm_c9.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, its frozen x86_64 and aarch64
header-closure records, and the selected consumer
`vendor/linux/drivers/phy/broadcom/phy-bcm-ns-usb2.c:51-67`.  This was manual
source inspection only; no compiler, formatter, analyzer, build, test, or
runtime command was used.

## Finding dispositions

1. **Parity P1 — SPDX identifier. Accepted and fixed.** Upstream line 1 is
   `SPDX-License-Identifier: GPL-2.0`; the candidate's `GPL-2.0-only` was an
   unauthorized change. The Rust provenance line now retains the exact
   upstream identifier.
2. **Parity P1 — nine unsuffixed integer literals. Accepted and fixed.** The
   nine macros at upstream lines 6-14 are unsuffixed hexadecimal integer
   literals, not `u32` declarations. All values (maximum `0x00000FFC`) are
   representable by signed `int`. The frozen target commands in
   `rewrite/FILE_MAP.tsv` bind this header to `x86_64-linux-gnu` and
   `aarch64-linux-gnu`; the pinned
   `include/uapi/asm-generic/int-ll64.h:26-27` identifies the targets'
   32-bit signed and unsigned integer types as `signed int` and `unsigned
   int`. The Rust constants consequently use `i32`, preserving the literal
   signed expression type rather than imposing unsigned fixed-width register
   types. The checked consumer's `usb2ctl &= ~MASK` at line 66 performs C's
   usual conversion of that signed `int` result to its `u32` operand; a later
   Rust translation must make that conversion at its expression site.
3. **Rust review acceptance of `u32`. Superseded.** It correctly found no
   ownership, unsafe, layout, allocation, configuration, or test concern, but
   its conclusion about the constants' unsigned expression type conflicts
   with the upstream unsuffixed literals and is rejected for that narrow
   integer-semantics point. No other Rust-review finding requires action.

## Final source-scope closure

- All nine selected operative macros for both frozen architectures are present
  with their exact names and values: the USB2 control offset, NDIV mask and
  shift, PDIV mask and shift, clock-set key, straps control offset, USB3 bit,
  and 4-byte bit.
- The include guard is a C preprocessing mechanism and has no Rust value,
  linkage, or ABI counterpart. The header defines no functions, types,
  statics, layouts, ownership/lifetime rule, lock/RCU/refcount operation, or
  conditional configuration branch.
- Every S013499 `PENDING_REVIEW` semantic entry is closed by this evidence:
  macro values and signed expression type are resolved; all remaining
  ownership, ABI, synchronization, lifetime, and configuration categories are
  not applicable to this constants-only header. No branding change remains.
