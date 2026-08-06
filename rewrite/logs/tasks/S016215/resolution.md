# Applier resolution — S016215

Task `S016215` translates
`include/uapi/linux/kernel-page-flags.h` to
`src/include/uapi/linux/kernel-page-flags.rs` for the frozen common scope
(x86_64 and aarch64), at pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

## Independent source recheck

I reopened all 40 lines of the pinned UAPI header, the candidate, the frozen
queue and scope records, the selected `fs/proc/page.c` consumer, and both
independent review reports. Lines 9--19 define `KPF_LOCKED` through
`KPF_BUDDY` as the uninterrupted values 0--10; lines 22--31 define
`KPF_MMAP` through `KPF_NOPAGE` as 11--20; and lines 33--38 define `KPF_KSM`
through `KPF_PGTABLE` as 21--26. The candidate declares exactly those 27
public constants in source order, spelling and value. No candidate correction
is required.

Every source replacement token is an unsuffixed decimal integer literal in
the C `int` range. The frozen x86_64 and aarch64 targets both use a signed
32-bit C `int`, so the candidate's `i32` constants preserve the relevant
signed value and shift-count surface consumed by `fs/proc/page.c`. The source
include guard at lines 2--3 and 40 controls repeated C preprocessing only; it
has no Rust declaration, storage, linkage, runtime, or ABI analogue. The
header has no configuration branch, function, type, object, allocation,
cleanup, locking, RCU/refcount, callback, ownership, lifetime, or driver ABI
contract.

## Review dispositions

1. Parity review: no findings. Accepted after independently verifying the full
   27-name/value inventory, source ordering, `KPF_ERROR` preservation, and the
   UAPI/non-UAPI split at the wrapper header.
2. Rust review: no findings. Accepted: immutable `i32` constants introduce no
   raw pointer, FFI, layout, aliasing, synchronization, panic, or `Drop`
   boundary.

## Semantic-record closure

- All 60 task-local `SYMBOLS.tsv` records are `COMPLETE`: the three
  include-guard records and 27 unconditional object-like macros for each
  frozen architecture. Each record identifies either the no-Rust-item guard
  treatment or its exact public `i32` value, and cites the pinned source lines
  2--40 and this adjudication. The S016215 `SCOPE.tsv` semantic status is
  likewise `COMPLETE`.
- `ABI.tsv`, `LIFETIMES.tsv`, `DRIVER_ABI.tsv`, and `BLOCKERS.tsv` contain no
  S016215 row. They are not applicable to this passive UAPI constants-only
  header; no unresolved layout, linkage, ownership, lifetime, locking, RCU,
  refcount, or driver-contract question remains.
- The destination retains the exact upstream `GPL-2.0 WITH Linux-syscall-note`
  SPDX expression and immutable source/revision/architecture/task provenance.
  It adds no branding delta, test, placeholder, unsafe code, or module-index
  change.

All five required task evidence files exist. No compiler, formatter, linker,
test, emulator, debugger, runtime command, or benchmark was run.
