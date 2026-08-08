# Parity review — S000758, attempt 2, slot 1

Result: APPROVE

Reviewed only `vendor/linux/arch/x86/include/asm/vmxfeatures.h`, the current
candidate, its current `candidate.diff`, frozen task records, and the narrow
Linux consumers in `arch/x86/kernel/cpu/feat_ctl.c` and `proc.c`.

- SPDX/provenance: Linux line 1 is `GPL-2.0`; candidate line 1 retains exactly
  `GPL-2.0`, and its immutable provenance identifies the pinned source,
  revision, x86_64 architecture, and S000758.
- Selected inventory: `SYMBOLS.tsv` selects the include guard, its `#ifndef` /
  `#endif`, `NVMXINTS`, and every `VMX_FEATURE_*` definition.  Linux lines
  8 and 17--92 provide 65 value macros; candidate lines 8--86 provide the
  same 65 public constants, with identical names.  No value macro or selected
  branch is omitted.
- Values and C expression type: each Linux value expression is formed from
  unsuffixed integer literals and therefore has signed C `int` arithmetic.
  Each candidate value is explicitly `i32`, preserving that signed 32-bit
  type and the original `word * 32 + bit` expression.  The defined values are
  0 through 100, so the preserved expressions have no signed-overflow edge.
  In particular, `NVMXINTS` remains 5, preserving the five-word/160-bit
  capacity used by `proc.c:13,112` and checked against
  `NR_VMX_FEATURE_WORDS` in `feat_ctl.c:30`.
- Guard/conditional behavior: the Linux header has no configuration-dependent
  feature branch; lines 2--3 and 93 are solely the C multiple-inclusion guard.
  The candidate is a single Rust module and adds no conditional behavior or
  duplicate definitions.  The header guard has no separate runtime, ABI, or
  externally linked symbol to preserve.
- ABI/mechanism/branding: this file defines no storage, functions, layout,
  allocation, locking, refcount, RCU, error, or linkage mechanism.  The
  candidate adds none and contains no branding delta.  The current
  `candidate.diff` is consistent with the attempt-2 signed-`i32` constant
  translation and introduces no additional behavior.

No parity findings.
