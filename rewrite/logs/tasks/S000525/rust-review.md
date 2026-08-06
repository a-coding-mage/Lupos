# Rust review — S000525

Reviewed only `vendor/linux/arch/x86/include/asm/extable_fixup_types.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, its mapped candidate
`src/arch/x86/include/asm/extable_fixup_types.rs`, the task row, and the
relevant extable/inline-assembly consumers.  No build, formatter, compiler,
linker, test, or runtime command was run.

## Findings

1. **P1 — SPDX provenance was changed.** The pinned source begins with
   `/* SPDX-License-Identifier: GPL-2.0 */`, while candidate line 1 says
   `// SPDX-License-Identifier: GPL-2.0-only`.  The rewrite protocol requires
   retaining the upstream SPDX identifier.  Restore the exact identifier (the
   Rust comment syntax may differ, but its SPDX value may not).

2. **P1 — `EX_DATA_REG`, `EX_DATA_FLAG`, and `EX_DATA_IMM` no longer retain
   their C macro argument/type semantics.** Upstream lines 14–16 are generic
   preprocessor expressions: the left operand's integer-promoted type governs
   the shift.  Candidate lines 22–41 replace each with a public `const fn`
   taking and returning `i32`.  Consequently valid upstream-style unsigned
   expressions (for example `EX_DATA_IMM(0xffffu)`) cannot be represented
   without a source-side cast, and a wider integral argument is prematurely
   narrowed instead of undergoing its C-contextual shift.  `wrapping_shl` also
   masks shift counts modulo 32, unlike the pinned C expression's compiler
   semantics for an out-of-range count.  The selected in-tree composed
   constants use only the signed `int` literals `-EFAULT`, `0`, `1`, `4`, and
   `8`, for which the candidate's resulting 32-bit bit patterns agree with the
   required `.long` exception-table data.  That does not make the translated
   exported function-like macros equivalent.  Preserve the frozen macro API's
   accepted integer domain and target-width behavior, or document a narrower
   mechanically proven interface in the task records before acceptance.

## Checked properties

- The four masks, three shift amounts, all register/flag/type constants, and
  all composed selected constants are present.  `EX_TYPE_EFAULT_REG` has the
  intended 32-bit two's-complement immediate-field bits for the pinned
  `EFAULT == 14` definition.
- `EX_DATA_IMM_MASK` is correctly represented as `u32`: C's `0xFFFF0000` is
  unsigned `int` on the frozen x86_64 target.  The other literal masks and all
  direct numeric type/field constants fit signed `int`.
- `struct exception_table_entry::data` is `int` in
  `arch/x86/include/asm/extable.h`; `arch/x86/mm/extable.c` later extracts the
  immediate with `FIELD_GET_SIGNED(EX_DATA_IMM_MASK, e->data)`.  This supports
  the candidate's chosen representation for the materialized table word, but
  not the type restriction of the three macro replacements.
- Candidate provenance source path, revision, architecture, and task ID match
  the queue and `vendor/linux.SHA`; only the SPDX value is incorrect.

Review outcome: **changes required**.  Both findings are source-only and are
appropriate for applier resolution; no Rust ownership, unsafe, layout, or
drop-timing issue exists in this constants-only candidate.
