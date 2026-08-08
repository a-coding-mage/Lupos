# Parity review — S000686 / slot 1

Reviewed only the pinned `arch/x86/include/asm/shared/tdx_errno.h`, the
candidate `src/arch/x86/include/asm/shared/tdx_errno.rs`, its candidate
snapshot, and frozen task records. No compiler, formatter, test, analyzer,
or historical Lupos source was used.

## Findings

1. **P1 — `_ASM_X86_SHARED_TDX_ERRNO_H` and all `TDX_*` macros: the C
   preprocessor/header contract is absent.** The selected source is a C
   header whose operative interface is the `#ifndef`/`#define`
   `_ASM_X86_SHARED_TDX_ERRNO_H` guard and 24 object-like `#define` macros
   (pinned source lines 3–40; each is selected in `rewrite/SYMBOLS.tsv`).
   The candidate provides Rust `pub const` items only. Rust items cannot be
   discovered through the C include/preprocessor interface, cannot implement
   the guard, and do not preserve an object-like macro's substitution behavior
   or its identifiers for C consumers. This is operative rather than
   cosmetic: pinned `arch/x86/include/asm/shared/tdx.h` includes this header
   before its `#ifndef __ASSEMBLER__` split (line 7), so the definitions are
   deliberately available in the C/assembly-facing portion of that shared
   header. The frozen records leave the guard and every listed macro
   `PENDING_REVIEW`; no source-proven generated-header or C-facing macro
   export mechanism appears in the reviewed candidate/frozen records.

2. **P1 — C literal-type and expression contract for operand-ID macros is
   not established.** `TDX_OPERAND_ID_RCX`, `TDX_OPERAND_ID_TDR`,
   `TDX_OPERAND_ID_SEPT`, and `TDX_OPERAND_ID_TD_EPOCH` are unsuffixed C
   hexadecimal integer macro expressions (pinned lines 35–38), whereas the
   candidate fixes them as `i32` constants. Those particular values fit the
   conventional signed `int` range, but replacing an untyped macro expression
   with a Rust item changes the language-level type/inference and eliminates
   use in a C preprocessor/C expression. The candidate and frozen records
   provide no ABI-bound wrapper or cross-language macro mapping that proves
   this change exact.

3. **P1 — the required candidate snapshot is incomplete.** The
   `candidate.diff` contains a prose assertion of translation rather than a
   focused source diff/snapshot. It therefore does not provide the required
   auditable candidate delta for the 24 selected macros, their exact values,
   C suffixes, or the header guard. This prevents an exhaustive candidate-diff
   parity review as required by the task protocol.

## Positive checks

The Rust file's provenance names the pinned source and exact revision, and
its architecture value is `x86_64`, matching the frozen task. The 20 ULL
status values and four operand-ID numerals are spelled with the same numeric
values under the same Linux names. That value preservation does not cure the
missing C/preprocessor interface above.

## Result

FINDINGS. Exact source parity cannot be accepted unless the missing
preprocessor/consumer mechanism and the candidate snapshot are established
from frozen source evidence.
