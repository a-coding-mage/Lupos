# Applier resolution — S012499

Reviewed the complete pinned `include/asm-generic/audit_change_attr.h` at
Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`, its four selected
inclusion sites, the candidate
`src/include/asm-generic/audit_change_attr.rs`, and both independent review
reports.

## Review dispositions

| Report | Finding | Disposition |
| --- | --- | --- |
| Parity review | No findings | Accepted. The four caller-supplied macro expansions retain every selected initializer value in upstream order, with consumer-owned storage and sentinel. |
| Rust review | No findings | Accepted. The declarative callback macros introduce no storage, FFI, ownership, layout, synchronization, panic, or unsafe boundary. |

## Independent applier checks

- The pinned SHA, destination provenance, and task mapping agree on
  `425f94c2954b1fe80ebdbf9b29854e89750355df`,
  `include/asm-generic/audit_change_attr.h`, `S012499`, and the `common`
  architecture scope.
- The upstream source is intentionally a reincludable comma-separated
  initializer fragment.  It owns neither array storage nor the following
  `~0U` terminator.  Each candidate macro preserves that boundary by invoking
  its caller-supplied macro once with the comma-terminated value sequence;
  the receiving translated caller continues to own its `u32` array and
  sentinel.
- Reopened inclusion sites are `arch/x86/kernel/audit_64.c:23-26`,
  `arch/x86/ia32/audit.c:11-14`, `lib/audit.c:22-25`, and
  `lib/compat_audit.c:22-25`.  The selected native/compat sequences are,
  respectively, 18, 21, 14, and 21 `unsigned int` values.  The candidate has
  precisely those counts, source order, membership, and explicit `u32`
  width.
- The frozen syscall-table context resolves every conditional exactly: native
  AArch64 omits `chmod`, `chown`/`lchown`, the `chown32` family, and `link`;
  its AArch32-compatible inclusion selects those entries.  x86_64 native
  omits only the legacy `chown32` family while its IA32 inclusion selects it.
  All four contexts select `fchown`, `setxattrat`, `removexattrat`,
  `fchownat`, `fchmodat2`, and `linkat`.
- The forty task-local `SYMBOLS.tsv` conditional records (ten directives for
  each frozen architecture) are closed with that concrete inclusion-context
  evidence.  This macro-only header has no task row in `ABI.tsv`,
  `LIFETIMES.tsv`, `DRIVER_ABI.tsv`, or `BLOCKERS.tsv`.
- The source retains its exact `GPL-2.0` SPDX expression and required
  immutable provenance.  It contains no tests, placeholder, replacement
  collection, artificial ABI object, unauthorized branding, or unsafe code.

No source change is required.  The candidate is accepted as the complete
fresh translation of this selected contextual initializer fragment.  No build,
formatting, test, linker, or runtime command was run.
