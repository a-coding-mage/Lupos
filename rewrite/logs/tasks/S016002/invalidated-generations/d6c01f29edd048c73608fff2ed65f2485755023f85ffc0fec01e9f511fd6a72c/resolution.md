# Applier resolution — S016002

Task `S016002` translates
`include/uapi/asm-generic/errno-base.h` to
`src/include/uapi/asm-generic/errno-base.rs` for the frozen common scope
(x86_64 and aarch64), at pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

## Independent source recheck

I reopened all 40 lines of the pinned header, the candidate, the frozen queue
and scope records, and both independent review reports.  Lines 5--38 define
exactly 34 unconditional object-like errno macros, in source order, from
`EPERM = 1` through `ERANGE = 34`.  The candidate declares the same 34 public
names in the same order with the same values.  Each source replacement token
is an unsuffixed decimal literal in the C `int` range; `core::ffi::c_int` is
the signed C `int` representation on both frozen LP64 targets.  Thus no value,
sign, width, or integer-literal behavior is changed.

The only conditional tokens are the C include guard at lines 2--3 and line
40.  They control repeated C preprocessing only and have no Rust declaration,
runtime, storage, linkage, or ABI analogue.  The header contains no functions,
types, objects, configuration-selected branch, allocation, cleanup, lock,
RCU/refcount, callback, or driver contract.  No destination source correction
is required.

## Review dispositions

1. Parity review: no findings.  Accepted after the complete 34-name/value
   inventory and include-guard treatment were independently rechecked.
2. Rust review: no findings.  Accepted: the immutable `c_int` constants create
   no raw pointer, FFI, layout, ownership, synchronization, panic, or `Drop`
   boundary.

## Semantic-record closure

- All 74 task-local `SYMBOLS.tsv` records are now `COMPLETE`.  They cover the
  include-guard condition and all 34 operative macros for each frozen
  architecture; each cites the identical unconditional source definitions at
  `vendor/linux/include/uapi/asm-generic/errno-base.h:2-40`, its exact macro
  value or no-Rust-item guard treatment, and this applier adjudication.  The
  S016002 `SCOPE.tsv` semantic-status record is likewise `COMPLETE`.
- `ABI.tsv`, `LIFETIMES.tsv`, `DRIVER_ABI.tsv`, and `BLOCKERS.tsv` contain no
  S016002 row.  Those record families are not applicable to this constant-only
  declaration header; no unresolved ABI, ownership, locking, RCU, refcount, or
  driver condition remains for this task.
- The destination retains the upstream UAPI SPDX expression and immutable
  source/revision/architecture/task provenance, adds no branding delta, test,
  placeholder, `unsafe`, or module-index change.

All five required task evidence files exist.  No compiler, formatter, linker,
test, emulator, debugger, runtime command, or benchmark was run.
