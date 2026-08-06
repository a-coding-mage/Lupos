# Parity review — S000555

Reviewed candidate `src/arch/x86/include/asm/inat_types.rs` against pinned
`vendor/linux/arch/x86/include/asm/inat_types.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, for frozen `x86_64`
configuration (`CONFIG_X86_64=y`, `CONFIG_64BIT=y`).  This was a manual,
source-only review; no compiler, formatter, analyzer, build, or test was run.

## Findings

1. **P1 — SPDX provenance was changed.**  The upstream file declares
   `SPDX-License-Identifier: GPL-2.0-or-later` at line 1, while candidate line
   1 declares `GPL-2.0-only`.  This narrows the upstream license identifier and
   violates the required retention of SPDX identifiers.  Restore the exact
   upstream identifier.

2. **P2 — selected operative include-guard macro has no preserved behavior.**
   Upstream lines 2–3 establish `_ASM_X86_INAT_TYPES_H`; both the conditional
   and macro are selected in `rewrite/SYMBOLS.tsv`.  The candidate has no
   equivalent exported/preprocessor contract or an evidence-backed disposition.
   In C, any consumer that tests this macro after including the header observes
   it as defined.  The applier must either preserve the required cross-language
   compile-time interface or explicitly resolve this selected macro's semantic
   record with pinned-source evidence; silently omitting it does not establish
   parity.

## Verified mappings

- `insn_attr_t`: `unsigned int` -> `u32`; the 32-bit attribute word is used by
  the inat table APIs and masks in `arch/x86/include/asm/inat.h`.
- `insn_byte_t`: `unsigned char` -> `u8`; decoder code uses it as instruction
  bytes and `insn.h` pairs it with four-byte fields.
- `insn_value_t`: `signed int` -> `i32`; `insn.h` overlays it with
  `insn_byte_t bytes[4]`, preserving the required 32-bit signed representation.

The three aliases have no C linkage, state, lifetime, locking, cleanup, or
configuration branch beyond this frozen x86_64 header selection.  Their Rust
names and widths/signs otherwise match the pinned declarations.

## Result

Rejected pending resolution of findings 1–2.
