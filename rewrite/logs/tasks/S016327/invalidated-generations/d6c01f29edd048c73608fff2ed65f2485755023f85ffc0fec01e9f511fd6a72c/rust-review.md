# Rust review — S016327

## Result

PASS — no Rust-specific correctness, ownership, ABI, or source-provenance
finding.

## Evidence reviewed

- Pinned source: `vendor/linux/include/uapi/linux/personality.h` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Candidate: `src/include/uapi/linux/personality.rs`.
- Task inventory records in `rewrite/{SCOPE,SYMBOLS,ABI,LIFETIMES,TRANSLATION_TASKS}.tsv`.
- Relevant pinned consumers, including the `unsigned int` `task_struct::personality`
  field and x86_64/AArch64, exec, mmap, mprotect, nommu, select, and syscall
  uses.

## Review

1. Both source anonymous enums have no named, stored, or FFI-visible enum
   type. Every enumerator value is representable by C `int`; their expressions
   therefore have signed-`int` value semantics. The candidate's `i32` constants
   preserve those exact values and bitwise expressions, including all low-byte
   personalities and the top-three-byte emulation flags. No expression can
   overflow `i32`.
2. `PER_CLEAR_ON_SETID` remains the same four-operand mask, evaluated as an
   `i32` constant. At the pinned consumers where it is combined with the
   `unsigned int` personality state, C would apply the usual unsigned conversion
   after this signed-`int` expression; the candidate does not claim a different
   storage or ABI type.
3. The constants are compile-time values only: the candidate introduces no
   allocation, references, ownership/lifetime state, `unsafe`, FFI layout,
   panic path, test configuration, placeholder, or synchronization behavior.
4. SPDX and immutable provenance identify the exact UAPI source, revision,
   common architecture scope, and task ID. The pinned source contains no
   additional copyright notice to retain, and no branding delta is present.
5. The source has no configuration-controlled semantic branch beyond its C
   inclusion guard, which has no Rust analogue. The candidate exports every
   selected value under its original name.

No source change is requested.
