# Resolution — S000686 / attempt 1 / P01

## Sources reopened

- Pinned `arch/x86/include/asm/shared/tdx_errno.h`, including its complete
  `_ASM_X86_SHARED_TDX_ERRNO_H` inclusion guard and all 24 selected object-like
  macros.
- Pinned `arch/x86/include/asm/shared/tdx.h:5-7,94-96`, which includes
  `tdx_errno.h` before the `__ASSEMBLER__` split.
- Direct pinned include sites: `arch/x86/virt/vmx/tdx/tdx.c:39`,
  `arch/x86/boot/compressed/mem.c:7`, and
  `arch/x86/boot/compressed/tdx.c:11` (through `tdx.h`).
- Frozen S000686 `SCOPE.tsv` and `SYMBOLS.tsv` records.  The scope record
  reports header-closure selection with 202 consumers; every selected guard
  and macro record remains `PENDING_REVIEW`.  No ABI, lifetime, driver-ABI,
  or blocker record supplies a Rust-to-C macro export or generated-header
  mechanism.

## Finding dispositions

1. **P1: missing C preprocessor/header contract — upheld, unresolved.**
   The source contract is not merely a collection of values: lines 3-4 and 40
   establish a textual C include guard and lines 6, 11-29, and 35-38 establish
   object-like macro substitutions.  `tdx.h` imports that contract before its
   `__ASSEMBLER__` conditional, so source evidence does not restrict it to a
   Rust-only module interface.  The sealed candidate has only Rust `pub const`
   items and the frozen records name no C-facing/generated-header bridge.
   Adding such a bridge would be a new mechanism outside the frozen mapping.

2. **P1: operand-ID literal expression/type contract — upheld, unresolved.**
   `TDX_OPERAND_ID_{RCX,TDR,SEPT,TD_EPOCH}` are unsuffixed C integer macro
   expressions at lines 35-38.  Their values fit `int`, but replacing the
   expressions with fixed Rust `i32` items does not preserve their C
   substitution and contextual conversion behavior.  The frozen records leave
   each of those operative macros `PENDING_REVIEW` and provide no exact Rust
   representation for that cross-language contract.  The Rust review's
   `i32` observation establishes numeric representability, not this missing
   interface mapping.

3. **P1: incomplete candidate snapshot — upheld.**
   `candidate.diff` is a prose statement and does not provide an auditable
   source delta for the guard or selected macros.  The candidate is sealed, so
   repairing that evidence would require a new candidate/review attempt; it
   cannot be silently repaired during application.

The Rust review's approval is retained as an ownership/type review of the
Rust-only constants.  It does not disprove the parity findings because it
identifies no frozen C/assembler export mechanism and no replacement mapping
for the selected preprocessor records.

## Outcome

The exact C preprocessor/assembly-visible macro contract cannot be established
as a Rust source mapping from the pinned source and frozen Phase 0 records.
This attempt is therefore **BLOCKED**.  No candidate source was changed and no
compiler, formatter, linker, analyzer, test, or runtime command was used.
