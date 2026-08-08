# S016368 resolution — P01 attempt 2

## Disposition

**CONTROLLED REQUEUE REQUIRED — do not mark `DONE` or `BLOCKED` from this
resolution.**  The current candidate is not accepted.  The missing macro
interface has a precise, in-scope source remedy, so the pinned source and
frozen records do not justify a blocker.  This applier made no source, queue,
semantic-ledger, or closure mutation.

Reviewed source-only inputs:

- `vendor/linux.SHA`: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- complete `vendor/linux/include/uapi/linux/securebits.h:1-83`
- `vendor/linux/include/linux/securebits.h:1-8`
- direct pinned uses in `vendor/linux/security/commoncap.c:805,994,1130,1167,1178,1382,1387,1394,1396,1424` and `vendor/linux/fs/open.c:399,431`
- current candidate and its candidate diff; both current review reports
- frozen `SCOPE.tsv`, `FILE_MAP.tsv`, `SYMBOLS.tsv`, `ABI.tsv`, `LIFETIMES.tsv`, and header-closure records

### F001 — `issecure_mask(X)` visibility

**Disposition: ACCEPTED; controlled requeue.**

The finding is correct.  The candidate's `macro_rules! issecure_mask` is
module-lexical and is used only by the candidate's own constant initializers.
It therefore does not provide the function-like interface defined at pinned
header line 9 to the frozen downstream wrapper, where
`include/linux/securebits.h:7` expands `issecure(X)` through
`issecure_mask(X)`.  The wrapper is independently frozen as task `S014935`
and declares `S016368` as a dependency.  The selected consumers also contain
direct calls, including `security/commoncap.c:994,1394,1396`.

The requeue is bounded: in the complete selected direct call context, every
evaluated `issecure_mask` argument is one of the header's named selectors,
whose pinned definitions are 0 through 10.  These are defined shifts of the
C `int` literal `1` in header line 9.  The replacement must expose a
cross-module Rust macro/interface for the later wrapper and consumers; it
must evaluate its argument once and use an explicitly `i32` left operand
(`1i32 << (X)`) for these selected C-`int` mask expressions.  The requeued
implementer must record the precise Rust visibility route used by S014935 and
the selected consumer translations, then both reviews must be repeated.

No blocker is warranted: the frozen scope contains the separate dependent
wrapper and every mechanically selected direct use is a defined, finite
selector use.  The pinned header supplies no defined behavior for shifts
outside the C `int` shift domain, so none may be invented while reworking the
selected mapping.

### R1 — Rust macro interface and type context

**Disposition: ACCEPTED; same controlled requeue as F001.**

The Rust review independently identifies the same omission and adds the
necessary type constraint.  The current unsuffixed `1` is inferred from each
local constant target; that preserves only the current internal constants,
not the source-level interface available to `include/linux/securebits.h` and
the selected callers.  Header line 9 has a C `int` left operand, and all
defined selected selector arguments are 0..10.  The controlled rework above
therefore requires the exported reusable mapping to retain an explicit `i32`
left operand and single evaluation, without substituting a fixed-value helper
or an untyped convenience API.

### F002 — `_UAPI_LINUX_SECUREBITS_H` include guard

**Disposition: DISPROVED as a missing runtime/source-interface behavior; no
Rust constant or macro is required.**

Pinned lines 2-3 and 83 show that `_UAPI_LINUX_SECUREBITS_H` exists solely to
prevent repeat textual preprocessing of this C header.  It is not read by any
of the pinned consumers and has no value, storage, layout, linkage, calling
convention, or callable interface.  The frozen scope gives this header one
unique destination, `src/include/uapi/linux/securebits.rs`, while the frozen
header closure records it as a provider for the selected translation units;
the future deterministic Rust module index supplies the one module inclusion
that corresponds to C's repeat-include suppression.  `ABI.tsv` and
`LIFETIMES.tsv` have no S016368 record requiring an ABI surface for the guard.

The mechanical conditional/guard rows in `SYMBOLS.tsv` remain evidence that
the C preprocessor structure was selected, not evidence that the guard name
must become a Rust API.  Their final semantic closure must record this mapping
when the controlled requeue is reviewed; it must not create a spurious public
guard item.

## Required next state

The coordinator should requeue the candidate for implementation/review using
the controlled requirements above.  This document is an adjudication record
only: it neither authorizes a `DONE` transition nor performs a queue
transition.
