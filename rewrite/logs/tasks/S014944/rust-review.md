# Rust review — S014944 (slot 2)

Reviewer role: Rust reviewer (`terra`, `high`)

Scope reviewed: `src/include/linux/sem_types.rs` against pinned
`vendor/linux/include/linux/sem_types.h`, plus the frozen configuration,
ABI/lifetime records, and pinned ownership context cited below.  This was a
manual source inspection only; no compiler, formatter, language server,
build, test, or debugger was invoked.  No source or queue file was changed.

## Preconditions

- The checked-out branch is `feat/bun-like-rewrite-test`.
- `vendor/linux.SHA` and `vendor/linux` `HEAD` both identify
  `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Queue row `S014944` is `REVIEWING`, belongs to `P01`, maps
  `include/linux/sem_types.h` to `src/include/linux/sem_types.rs`, and has
  architecture class `common`.

## Findings

No Rust ownership, aliasing, layout, or FFI finding.

1. The candidate's provenance identifies the exact task, pinned source,
   revision, and common architecture scope
   (`src/include/linux/sem_types.rs:1-5`).  Its `sysv_sem` is `#[repr(C)]` and
   contains exactly one `*mut sem_undo_list` field
   (`src/include/linux/sem_types.rs:23-27`).  The pinned header declares the
   same one pointer field (`vendor/linux/include/linux/sem_types.h:7-10`).
   `CONFIG_SYSVIPC=y` in both frozen configurations
   (`rewrite/configs/x86_64/frozen.config:55`,
   `rewrite/configs/aarch64/frozen.config:40`), so retaining that field for
   both supported architectures is correct.  A C-layout Rust struct holding
   one raw pointer preserves the required pointer-sized field layout and does
   not introduce Rust-managed ownership or drop behavior.

2. The pinned header only forward-declares `struct sem_undo_list`
   (`vendor/linux/include/linux/sem_types.h:5`); the candidate represents it
   as a `#[repr(C)]` zero-field opaque type with an inaccessible field
   (`src/include/linux/sem_types.rs:7-17`).  There is no by-value
   `sem_undo_list` member in this header, only the pointer above.  Its concrete
   pinned definition contains a refcount, spinlock, and intrusive list
   (`vendor/linux/ipc/sem.c:159-166`), so keeping it opaque here avoids an
   unsupported Rust ownership or layout assertion while retaining a valid
   pointer target type for FFI.

3. The raw mutable pointer faithfully keeps Linux's nullable, non-owning,
   aliased pointer representation.  The candidate creates neither Rust
   references nor `Drop` logic, and therefore does not assert exclusive access
   or alter destruction timing.  In the pinned implementation the pointer is
   allocated/initialized and stored in the task (`vendor/linux/ipc/sem.c:1850-1866`),
   may be shared with a child after a refcount increment
   (`vendor/linux/ipc/sem.c:2302-2318`), and is nulled before a refcounted
   release path (`vendor/linux/ipc/sem.c:2335-2345`).  The header-level raw
   pointer model is consequently the appropriate boundary; locking, refcount,
   and RCU behavior belong to the defining implementation rather than this
   forward-declaration header.  Pointer identity is also externally observed
   without dereference (`vendor/linux/kernel/kcmp.c:197-204`), which the raw
   pointer preserves.

4. Frozen ABI and lifetime rows for both architectures still show
   `PENDING_REVIEW` (`rewrite/ABI.tsv:152585-152586`,
   `rewrite/LIFETIMES.tsv:148526-148527`).  Source evidence resolves the
   header-specific facts as: `sysv_sem` is a C-layout single nullable raw
   pointer; it has no direct ownership, lifetime, locking, RCU, or refcount
   operation in this header; and `sem_undo_list` remains opaque at this
   boundary.  The applier must record those dispositions under the required
   Phase-1 process; this is an evidence-record closure item, not a candidate
   source defect.

## Verdict

Accepted from the Rust source-review perspective.  No candidate source change
is requested.
