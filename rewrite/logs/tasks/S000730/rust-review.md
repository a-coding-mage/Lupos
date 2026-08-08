# Rust review — S000730 / attempt 1 / P02 / slot 2

Reviewer: rust_reviewer (`gpt-5.6-terra`, high). Scope was limited to the
pinned `arch/x86/include/asm/trapnr.h`, the candidate snapshot, frozen task
records, and direct pinned consumers. No compiler, formatter, test, runtime,
or analyzer was invoked.

## Result: FINDINGS

### R1 — typed Rust constants do not preserve the header's preprocessor and assembler contract

The candidate changes every `#define` (and the `_ASM_X86_TRAPNR_H` include
guard) into a Rust `pub const ...: i32`. The numerical values are equal, but
the C source does not define Rust items: it defines replacement tokens usable
by both the C preprocessor and the assembler. Pinned
`arch/x86/entry/entry_64.S:332` and `:347` use `X86_TRAP_BP` in assembler
`.if` expressions. Pinned `arch/x86/include/asm/vmx.h:423-430` derives
`INTR_TYPE_*` by preprocessor-shifting `EVENT_TYPE_*`; the corresponding
values are passed at unsigned-integer call boundaries, e.g.
`arch/x86/kvm/vmx/vmx.c:7178` to `x86_entry_from_kvm(unsigned int, unsigned
int)`. A Rust item cannot supply either macro expansion/assembler use or C's
contextual integer conversion. Its fixed `i32` type further changes the
interface from the header's `int` literal tokens and requires explicit Rust
casts at unsigned/bitfield consumers.

No source-proven bridge that preserves these selected macros and their
include-guard behavior for the original C/assembly consumers is present in
the candidate or frozen records. The review therefore rejects the proposed
semantic closure and requires a source-proven cross-language macro/assembly
mechanism; it must not be papered over by an ordinary Rust constant module.

Affected closure records: the guard and all selected operative macro
`selection_expression` records, including the event-type and trap-number
families. There are no candidate pointers, references, unsafe blocks, FFI
layouts, allocation paths, or drop/Send/Sync behavior to approve separately;
the blocking defect is the missing language-boundary mechanism itself.

## Evidence

- Linux source: `vendor/linux/arch/x86/include/asm/trapnr.h:2-44`.
- Direct consumers: `vendor/linux/arch/x86/entry/entry_64.S:332-347`,
  `vendor/linux/arch/x86/include/asm/vmx.h:423-430`, and
  `vendor/linux/arch/x86/kvm/vmx/vmx.c:7178`.
- Candidate snapshot SHA-256:
  `0a1b65da068d7722f44331e4c78c9357524a9b4584cca4046d77f6d71079019d`.
