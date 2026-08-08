# S000772 applier resolution

Pinned source reopened: `vendor/linux/arch/x86/include/uapi/asm/debugreg.h`
at `425f94c2954b1fe80ebdbf9b29854e89750355df`.  The frozen row is
`S000772`, `arch/x86/include/uapi/asm/debugreg.h` to
`src/arch/x86/include/uapi/asm/debugreg.rs`, for `x86_64` only.

## Finding dispositions

1. Parity review, slot 1: **no finding**.  Disposition: accepted.  Reopening
   the complete header confirms every selected `DR_*` macro is represented:
   the 27 ordinary C `int` literals are `i32`, `DR6_RESERVED` is the
   source-selected `unsigned int` (`u32`), and the selected non-`__i386__`
   `DR_CONTROL_RESERVED` `UL` literal is `u64`.  `DR_TRAP_BITS` retains the
   four source operands and their OR expression.  The excluded `__i386__`
   branch is outside this x86_64 task.  Direct pinned uses in
   `arch/x86/kernel/hw_breakpoint.c` and `arch/x86/kernel/ptrace.c` preserve
   their original shifting, masking, and unsigned-long promotion behavior;
   this header adds no storage, control flow, ABI object, or synchronization
   operation.
2. Rust review, slot 2: **no finding**.  Disposition: accepted.  The candidate
   contains immutable scalar constants only: no references, ownership,
   allocation, layout, FFI, `unsafe`, panic, or drop behavior is introduced.
   The two unsigned types above preserve the only source literal-width
   distinctions required by the selected x86_64 C expressions.

No source edit is warranted: the candidate already matches the pinned
x86_64 UAPI definitions and adding a redesign would be unsupported.  The
task-owned `PENDING_REVIEW` records are closed only through the reviewed
semantic-closure transaction; its field-level final and disposition evidence
is generated and committed after this resolution.  No compiler, formatter,
linker, test, runtime, or historical Rust source was used.
