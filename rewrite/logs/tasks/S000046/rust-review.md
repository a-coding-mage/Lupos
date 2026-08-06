# S000046 Rust review (slot 2)

**Verdict: REJECT — one blocking Rust/AArch64 ABI finding.**

Reviewed candidate: `src/arch/arm64/include/asm/compiler.rs` for
`vendor/linux/arch/arm64/include/asm/compiler.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

## Finding R1 — `x30` cannot be used as a Rust inline-assembly operand

**Severity: blocking.**  The candidate's `xpaclri!` macro passes the AArch64
link register as `inout("x30")` (candidate lines 29–33).  `x30` is the
AArch64 ABI link register and is reserved from Rust inline-assembly operands;
the candidate therefore does not provide a valid Rust representation of the
GNU-C local register variable used by the Linux macro.  Even a backend that
were to admit such an operand would require a stated and verified contract for
preserving the enclosing Rust function's return address.  No frozen Rust
toolchain/inline-assembly ABI record supplies that contract.

The pinned Linux source deliberately uses a GNU-C explicit register variable,
`register unsigned long __xpaclri_ptr asm("x30")`, with a tied `"+r"`
constraint before executing `hint #7` (compiler.h:11–19).  The fixed register
is semantic: XPACLRI operates on `x30`, so replacing this operand with a
general Rust register would not preserve the macro.  Active consumers pass
saved LR/PC values through the stripping macros (for example
arch/arm64/kernel/stacktrace.c:275 and :534, and kernel/process.c:224), so the
invalid operand is on selected behavior rather than unreachable header text.

The applier must establish a source-backed, Rust-legal AArch64 ABI mechanism
that both applies XPACLRI to the supplied value and preserves the caller's
link-register/return control flow, then record its ABI/unsafe contract.  If no
such mechanism exists within the frozen file scope, this task must be
`BLOCKED`; it must not retain `inout("x30")` or substitute a general-register
operation.

## Checked items without findings

- The retained AArch64 command defines `ARM64_ASM_ARCH="armv8.5-a"` and uses
  `-Wa,-march=armv8.5-a`; the candidate's active `.arch armv8.5-a\n` preamble
  matches that frozen selection.
- `CONFIG_ARM64_PTR_AUTH=y` and `CONFIG_ARM64_PTR_AUTH_KERNEL=y` select both
  XPACLRI stripping expansions.  `CONFIG_BUILTIN_RETURN_ADDRESS_STRIPS_PAC=y`
  leaves the Linux return-address builtin unwrapped, which the candidate does
  not replace.
- The candidate evaluates the macro operand once and returns `usize`, which
  has the same width as Linux `unsigned long` for the frozen AArch64 target.
  Its `nomem`, `nostack`, and `preserves_flags` assertions match the source
  instruction's lack of memory, stack, and condition-code operands; they do
  not cure R1's reserved-register/return-address violation.
- This header defines no storage, ownership relation, `Drop` path, aggregate
  layout, or FFI-visible data structure beyond the macro/assembly contract.

No compiler, formatter, linker, test, runtime, or benchmark command was run.
