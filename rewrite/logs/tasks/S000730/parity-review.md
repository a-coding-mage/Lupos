# Parity review — S000730 / P02 / attempt 1

**Reviewer:** parity reviewer (`gpt-5.6-terra`, high)

**Verdict:** FINDINGS — do not accept this candidate as a source-equivalent
translation.

## Findings

1. **P1 — all operative `EVENT_TYPE_*` and `X86_TRAP_*` macros lose their
   preprocessor/assembler-token contract.**

   `arch/x86/include/asm/trapnr.h:8-42` defines 32 object-like C macros, not
   linkable or namespaced data objects.  The candidate turns every one into a
   Rust module item (`pub const …: i32`).  Consequently the names cannot be
   expanded by the C preprocessor, used in assembler conditionals/immediates,
   or composed into later C macros.  This is exercised directly by pinned
   source: `arch/x86/entry/entry_64.S:39,332,347` includes this header and
   compares `X86_TRAP_BP` in `.if`; `arch/x86/boot/compressed/mem_encrypt.S:16,222`
   includes it and emits `$X86_TRAP_VC`; and
   `arch/x86/include/asm/vmx.h:423-430` composes every `EVENT_TYPE_*` name in
   `INTR_TYPE_*` macro expressions.  The fixed Rust type also cannot preserve
   macro substitution in an arbitrary C integer context.  The source and
   frozen ABI records provide no Rust-to-C/assembly macro export mechanism or
   approved interface that could make these `pub const`s equivalent.

2. **P2 — the selected include-guard conditional and operative guard macro
   are omitted.**

   The frozen symbol inventory selects `ifndef@2`, `_ASM_X86_TRAPNR_H` at
   line 3, and `endif@44`; pinned `trapnr.h:2-3,44` uses them to make repeated
   C/assembly inclusion idempotent.  The candidate has no representation for
   the guard and no proven generated-module integration which supplies the
   same preprocessing behavior.  This is distinct from merely making Rust
   module loading idempotent: the original guard controls the token stream of
   the native C and assembly consumers above.

All numeric spellings and all 32 numeric values in the Rust file match the
pinned definitions, but that does not resolve either interface difference.
The task's `ABI.tsv` and `LIFETIMES.tsv` have no record establishing a
replacement cross-language/header contract.  Exact parity is therefore not
source-provable from the frozen inputs.
