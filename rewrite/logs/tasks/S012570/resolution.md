# Resolution — S012570

Applier P02 reopened the complete pinned
`vendor/linux/include/asm-generic/percpu_types.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the final candidate, both
independent review reports, the frozen task/symbol/header-closure records,
`include/linux/compiler_types.h`, and x86's
`arch/x86/include/asm/percpu_types.h`.  This was source-only adjudication: no
compiler, formatter, linker, test, runtime command, diagnostic, or historical
Lupos Rust source was used.

## Review disposition

Both reviews pass and require no source change.  The 19-line oracle has no C
object, function, type, storage, linkage, layout, or executable operation.  In
a non-assembly preprocessing context its sole non-guard effect is the generic
fallback `#define __percpu_qual` with an empty replacement list, and only when
an architecture did not define it first.  The item-free Rust module preserves
that absence without inventing a Rust item, macro API, named-address-space
marker, data symbol, ownership contract, or ABI surface.

The source-level context confirms the boundary: `compiler_types.h` uses the
token solely as part of the C-only `__percpu` qualifier, while the x86 wrapper
owns any conditional architecture override before including this generic
header.  The generic fallback cannot override such an existing definition.
The frozen header closure selects this path for 2,918 x86_64 and 8,902 AArch64
consumers; neither selection adds an object-level declaration to this header.

## Final semantic-record closure

All S012570 `PENDING_REVIEW` symbol records are closed in this task evidence
for both frozen architectures:

- `_ASM_GENERIC_PERCPU_TYPES_H_` and all include-guard `#ifndef`/`#endif`
  records are preprocessing-only include-once control, with no Rust runtime,
  ownership, lifetime, layout, linkage, or ABI counterpart.
- The `__ASSEMBLER__` conditional is a C/assembly preprocessing boundary.  A
  Rust source module is never an assembler include and therefore represents no
  branch-specific Rust behavior.
- `__percpu_qual` is an empty generic preprocessor replacement list, only when
  not supplied by an architecture wrapper.  It creates no type, storage,
  value, symbol, cleanup, synchronization, RCU, refcount, or ABI contract;
  hence its exact Rust mapping is no item.

There are no S012570 ABI, lifetime, driver-ABI, or blocker rows requiring a
separate representation decision.  The task's frozen Phase 0 TSVs remain
authoritative evidence; this resolution supplies the required task-local
closure rather than altering them.  The candidate is accepted for `DONE` as a
source-pipeline result only, with no build, link, test, boot, or runtime claim.
