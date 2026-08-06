# S000496 parity review — slot 1

## Verdict

**REJECT: one source-provenance defect.** No feature- or bug-index formula defect was found.

## Finding P1 — SPDX identifier was changed

- **Pinned source:** `vendor/linux/arch/x86/include/asm/cpufeatures.h:1` is `/* SPDX-License-Identifier: GPL-2.0 */`.
- **Candidate:** `src/arch/x86/include/asm/cpufeatures.rs:1` is `// SPDX-License-Identifier: GPL-2.0-only`.
- **Impact:** The translation changes the upstream SPDX identifier. The rewrite protocol requires retaining SPDX identifiers, and this task's immutable provenance must reflect the pinned source. `GPL-2.0-only` may describe a related license expression, but it is not the identifier in the authoritative source.
- **Required resolution:** Preserve the exact upstream SPDX identifier (`GPL-2.0`) in the Rust source header.

## Exhaustive comparison performed

- Pinned revision and candidate provenance revision both resolve to `425f94c2954b1fe80ebdbf9b29854e89750355df`; source path, task ID, and `x86_64` architecture provenance otherwise match.
- The header contains 425 `X86_FEATURE_*` object macros and 43 `X86_BUG_*` object macros. The candidate has all 425 feature constants and all 42 active-x86_64 bug constants. The omitted `X86_BUG_ESPFIX` is correct: it is the sole definition behind `#ifdef CONFIG_X86_32`, while the frozen configuration sets `CONFIG_X86_64=y` and has no `CONFIG_X86_32` selection.
- Every active object-like macro name and its `word * 32 + bit` / `X86_BUG(...)` formula agrees with the pinned header. The candidate has no duplicate feature indices, and preserves `NCAPINTS == 22`, `NBUGINTS == 2`, and the two bug words.
- The function-like `X86_BUG(x)` is represented by a public `const fn` computing `NCAPINTS * 32 + x`; all pinned uses in this header are covered. The include guard has no Rust analogue and is correctly not represented as a runtime declaration.
- There are no additional configuration guards, ABI/linkage declarations, branding differences, placeholders, or Rust test configuration in the candidate.

No source, queue, manifest, or non-review evidence file was modified by this reviewer. No build, compiler, formatter, test, or runtime command was run.
