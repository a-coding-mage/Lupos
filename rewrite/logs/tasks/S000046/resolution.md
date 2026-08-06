# S000046 applier resolution

Verdict: **BLOCKED**.  No source change is accepted for this task.

## P1 — `ARM64_ASM_PREAMBLE` composition

Accepted in part: the frozen AArch64 commands select
`ARM64_ASM_ARCH="armv8.5-a"` and `-Wa,-march=armv8.5-a`, so the C preamble
used by `xpaclri` is exactly `.arch armv8.5-a\n`.  The candidate's nested
`macro_rules!` invocation is not evidence that Rust accepts the C-style
adjacent-template composition used by the selected `__TLBI_*` and
`__MTE_PREAMBLE` consumers.  This cannot be repaired independently here
because its required composition contract crosses the selected headers
`asm/tlbflush.h` and `asm/mte-def.h`.

This finding does not by itself determine the terminal state, because the
candidate could have placed the fixed selected preamble and `hint #7` in one
literal template at this call site.  It remains unresolved pending the
scope-level assembly-template representation described below.

## R1 — fixed `x30` input/output and link-register preservation

Accepted and blocking.  `compiler.h:11-19` uses a GNU-C local register
variable explicitly fixed to `x30` and a tied `+r` operand.  `hint #7`
(XPACLRI) operates on `x30`; changing it to a general-register operand would
not execute the selected source operation.  The frozen configuration enables
both `CONFIG_ARM64_PTR_AUTH` and `CONFIG_ARM64_PTR_AUTH_KERNEL`, and the
selected consumers pass saved PC/LR values through the macro, including
`arch/arm64/kernel/stacktrace.c:275`, `arch/arm64/kernel/stacktrace.c:534`,
and `arch/arm64/kernel/process.c:224`.

The candidate's `inout("x30")` is rejected.  The frozen Phase 0 identity pins
the C compiler, target, configuration, and Kbuild flags, but contains no
frozen Rust compiler/inline-assembly ABI record that legalizes `x30` as an
inline-assembly operand or specifies how an enclosing Rust function's link
register and return control flow are preserved across this operation.
Writing `mov x30, ...` in an otherwise general-register template would leave
that mutation unmodelled to Rust; an unrecorded out-of-line assembly helper
would add a source/mapping/ABI dependency outside this task.  Neither is a
source-faithful mapping established by the pinned evidence.

The rejected candidate has been removed so the tree contains no guessed or
known-invalid `x30` inline assembly.

## Exact prerequisite for reopening

Reopen Phase 0/scope and provide both of the following pinned, auditable
records before requeuing this file:

1. a Rust-target inline-assembly ABI contract, for the selected AArch64 Rust
   toolchain and target, that proves a legal input/output mapping for the
   XPACLRI `x30` operation while preserving the enclosing function's link
   register and return control flow; or a source-mapped assembly shim whose
   Linux-path classification, `FILE_MAP`, symbol, ABI, and caller contract
   prove the same behavior; and
2. a selected-header assembly-template composition representation for
   `ARM64_ASM_PREAMBLE` that preserves its `.arch armv8.5-a\n` concatenation
   role for the selected TLBI and MTE users.

No compiler, formatter, linker, test, runtime, or benchmark command was run.
