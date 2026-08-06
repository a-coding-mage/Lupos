# Parity review — S014640

Reviewed `src/include/linux/pid_types.rs` against the complete pinned
`vendor/linux/include/linux/pid_types.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` for the frozen common
x86_64/aarch64 scope. The queue row was `REVIEWING` on P01 when reviewed.
Source-only inspection included the selected symbol/ABI/lifetime records,
both frozen configurations, and the direct authoritative consumers and
definition context in `include/linux/pid.h`, `include/linux/pid_namespace.h`,
and `kernel/pid.c`. No build, formatter, analyzer, test, or debugger was run.

## Findings

1. **MAJOR — `pid_namespace` is defined here as a zero-sized concrete Rust
   type, but the Linux header only forward-declares an incomplete C type.**
   `pid_types.h:13-14` deliberately supplies only `struct pid_namespace;` and
   an extern declaration. The actual, nonzero, configuration-dependent type
   is defined in `include/linux/pid_namespace.h:26-50`; with the frozen
   `CONFIG_PID_NS=y` and `CONFIG_SYSCTL=y` configurations, it contains the
   namespace state and fields used by `kernel/pid.c:72-84` to define and export
   `init_pid_ns`. `pid_types.rs:28-32` instead makes this header own a
   `#[repr(C)]` ZST. That changes an incomplete declaration into a complete,
   zero-sized nominal type and prevents `init_pid_ns` from sharing the one
   canonical complete `pid_namespace` layout required by the separate selected
   `include/linux/pid_namespace.h` translation task (S014639). In particular,
   consumers that legitimately obtain `&init_pid_ns` and access its fields,
   such as `pid_namespace.h:142-145` and `pid.h:318-326`, cannot retain their
   C type/layout contract. Preserve a single canonical `pid_namespace` type
   at its defining-header translation and make this header’s import reference
   that type without inventing a ZST definition.

2. **MAJOR — `pid_type` is modeled as a Rust closed enum, which rejects C’s
   scalar representation domain at ABI boundaries.** Linux exposes
   `enum pid_type` by value and by pointer: see `include/linux/pid.h:95-113`,
   `pid.h:232`, and `include/linux/sched/signal.h:286`. C’s enum object uses
   its compatible integer representation; the source performs integer-indexed
   operations on it (`kernel/pid.c:380-419`) and has no Rust-style invalid
   discriminant invariant. `pid_types.rs:13-19` makes every non-0..4 bit
   pattern invalid Rust data, so receiving/storing an out-of-range C enum
   representation through those APIs is undefined in Rust rather than
   preserving the original scalar behavior. Model the ABI as its explicit
   compatible integer-width newtype/constant set (with the frozen target ABI
   established from the Phase 0 record), retaining all five values and the
   integer domain rather than introducing a closed Rust enum.

3. **MINOR — `pid_type` lacks the trivial-copy semantics of the C enum.**
   The source enum is a scalar and is copied freely through the call and array
   indexing sites above. The candidate at `pid_types.rs:13-19` derives neither
   `Copy` nor `Clone`, making a Rust value move-only. Whichever integer/newtype
   representation resolves finding 2 must preserve trivial by-value copying.

## Checked parity items

- All five source enumerators are present in source order with values 0..4.
- The imported symbol is named `init_pid_ns`, is externally declared, and is
  mutable, matching the writable exported Linux object; its type issue is
  covered by finding 1.
- Both configurations select this header; no source conditional changes the
  five enumerators or the forward/external declarations.
- Required provenance is present, carries the pinned revision, records
  `common`, and names S014640. No unauthorized branding, test configuration,
  or placeholder macro was observed.

Result: **changes required; do not accept until the findings are resolved and
the pending task ABI/lifetime records are closed by the applier.**
