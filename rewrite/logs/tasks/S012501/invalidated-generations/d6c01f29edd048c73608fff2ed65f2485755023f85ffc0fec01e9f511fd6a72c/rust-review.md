# Rust review — S012501

Reviewer: Rust reviewer (Terra, high)  
Pipeline: P02  
Scope: `include/asm-generic/audit_read.h` -> `src/include/asm-generic/audit_read.rs`

## Verdict

CHANGES_REQUESTED.

## Evidence reviewed

- Pinned source `vendor/linux/include/asm-generic/audit_read.h:2-20`.
- Selected inclusion contexts: `lib/audit.c:12-15`, `lib/compat_audit.c:12-15`,
  `arch/x86/kernel/audit_64.c:13-16`, and `arch/x86/ia32/audit.c:21-24`.
- Frozen scope/closure and task dependency records for both x86_64 and AArch64.
- Candidate `src/include/asm-generic/audit_read.rs:7-64`.
- The consuming ABI is an `unsigned int *` list, terminated by the caller's
  `~0U` (`kernel/auditfilter.c:168-187`).

## Findings

### R1 — high: exported callback macros are not an equivalent representation of the C initializer fragment

The upstream header deliberately has no declared API or include guard.  At
each inclusion it injects comma-separated expressions directly into the
surrounding `unsigned`/`unsigned int` array initializer; the includer's active
`__NR_*` definitions select the guarded expressions.  The candidate instead
declares four new crate-root public macros (`#[macro_export]` at lines 11, 25,
41, and 56), each requiring an `ident` naming a callback macro.

This changes both visibility and use semantics: it creates public names that
the C source does not export, exposes every architecture variant on every
build, and cannot be used directly where the C fragment is used.  It also
restricts a consumer to a single identifier callback, rather than permitting
the array-initializer expression context present at all four selected call
sites.  The documentation does not restore this missing compile-time/context
contract.

Required resolution: use a private, configuration-accurate representation
whose selected consumers can form their exact `[u32; N]` initializer (including
the caller-owned `u32::MAX` terminator) without adding exported Rust API or a
new callback protocol.  Preserve the source-local/static visibility of the
four owner arrays when their owner files are translated.  If a macro is
unavoidable, keep it non-exported and make its invocation semantics a direct,
expression-list substitute for the relevant selected owner rather than a
public callback interface.

### R2 — medium: architecture configuration is encoded as unchecked API selection

The C `#ifdef`s are evaluated from the syscall header included by each owning C
translation unit.  Thus the same header has four selected outputs under the
frozen configurations: x86 native, x86 IA32, AArch64 native, and AArch64
compat.  The candidate relies on future callers choosing one of four
architecture-labelled exported macro names.  Nothing constrains that choice by
the actual target or selected ABI, so an x86 list can be selected in an
AArch64 translation unit (or vice versa), an invalid state that the C include
cannot produce in these contexts.

Required resolution: bind the chosen representation to the owning target/ABI
at the owner translation boundary (or use target configuration selection that
has the same effect).  Do not leave ABI selection to unconstrained external
macro-name choice.

## Confirmed source facts for resolution

- The candidate's four numeric sequences match the four selected C inclusion
  contexts.  All values are non-negative and fit the required 32-bit
  `unsigned int` element type on x86_64 and AArch64.
- `~0U` is not emitted by this header; it is correctly identified as belonging
  to each surrounding C array initializer.  Any owner-side Rust array must
  retain its `u32::MAX` terminator, because `audit_register_class()` scans until
  that value.
- This header defines no FFI symbol, layout, callable function, ownership
  operation, or unsafe operation.  Its only material contract is exact
  compile-time expression-list expansion and the element order/type consumed
  by the four static arrays.
- All sixteen S012501 conditional rows in `rewrite/SYMBOLS.tsv` remain
  `PENDING_REVIEW`.  The applier must explicitly close the conditional facts:
  `__NR_readlink`, `__NR_listxattrat`, `__NR_getxattrat`, and
  `__NR_readlinkat` for both architectures, including their selected inclusion
  contexts, before `DONE`.

No compiler, formatter, test, linker, rust-analyzer diagnostic, or runtime
tool was used in this review.
