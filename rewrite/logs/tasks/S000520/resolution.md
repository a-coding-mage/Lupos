# Applier resolution — S000520 (attempt 1)

## Disposition

**BLOCKED.** The candidate cannot be accepted.  The frozen one-file mapping
does not provide an audited Rust macro/assembler representation that preserves
the pinned header's use-site token-expansion contract.

## Source evidence

The complete pinned `arch/x86/include/asm/emulate_prefix.h:1-14` has SPDX
identifier `GPL-2.0` (not `GPL-2.0-only`).  Its only operative payload is two
object-like preprocessor expansions:

- `__XEN_EMULATE_PREFIX` at line 11 expands to the five untyped comma-separated
  tokens `0x0f,0x0b,0x78,0x65,0x6e`.
- `__KVM_EMULATE_PREFIX` at line 12 expands to the five untyped comma-separated
  tokens `0x0f,0x0b,0x6b,0x76,0x6d`.

The exact consumers establish that these are not byte-array objects:

- `arch/x86/lib/insn.c:82-83` places each expansion inside an
  `insn_byte_t` array initializer.
- `arch/x86/kvm/x86.c:8024` places the KVM expansion inside a `char` array
  initializer.
- `arch/x86/include/asm/xen/interface.h:387` embeds the Xen expansion in
  `__ASM_FORM(.byte __XEN_EMULATE_PREFIX ;)`.  In the pinned
  `arch/x86/include/asm/asm.h:8-16`, `__ASM_FORM` is context-sensitive:
  assembler mode preserves assembler tokens, while C mode stringifies them for
  inline assembly.

The frozen x86_64 configuration includes `CONFIG_X86_64=y`,
`CONFIG_PARAVIRT=y`, and `CONFIG_KVM_GUEST=y`; no condition in this header
removes either operative macro.  The Phase 0 symbol records for the include
guard and both macros remain `PENDING_REVIEW`; there are no ABI or lifetime
rows because this header declares no object, type, or function.

## Review finding dispositions

1. **Parity P1 — accepted.** Candidate `pub const ...: [u8; 5]` items are
   typed Rust array expressions.  They cannot expand as five caller-owned
   initializer elements or as operands of the assembler `.byte` form, and
   therefore do not preserve either macro's interface.
2. **Rust finding 1 — accepted.** Rust's invocation syntax and typed expression
   model cannot transparently substitute for these C object-like comma-token
   expansions.  No source-local mechanism in the frozen mapping binds a Rust
   macro or inline-assembly form to all three demonstrated contexts.
3. **Rust finding 2 — accepted.** Candidate line 1 changes the exact upstream
   SPDX identifier from `GPL-2.0` to `GPL-2.0-only`.  Correcting that identifier
   alone cannot cure the rejected macro representation.

## Why no source change was applied

Replacing the candidate arrays with another public Rust array, a callback/X
macro convention, or a new inline-assembly helper would create an unaudited
interface that cannot be invoked in the original C initializer and assembler
positions.  Editing future consumer destinations or introducing shared macro
machinery would exceed this leased one-file task and is not specified by the
frozen file map, symbol records, ABI records, or dependencies.

The candidate remains rejected.  Its task-local `PENDING_REVIEW` semantic
records cannot be closed as `COMPLETE`; this resolution closes their review
disposition as a concrete blocker.  A later scope/translation-design decision
must define and review an exact context-bound mechanism for C comma-token
macros and assembler operands before this task is requeued.

No compiler, formatter, linker, test, emulator, debugger, benchmark, or
rust-analyzer diagnostic was used.
