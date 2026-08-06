# Applier resolution — S014944

Applied by the independent applier against pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

## Reopened authoritative context

The complete pinned `include/linux/sem_types.h` has a forward declaration of
`struct sem_undo_list` and, only when `CONFIG_SYSVIPC` is enabled, defines
`struct sysv_sem` with exactly one member:

`struct sem_undo_list *undo_list;`

Both frozen configurations set `CONFIG_SYSVIPC=y`, so that sole field is
selected for both x86_64 and AArch64.  `sched.h` embeds this aggregate as
`task_struct.sysvsem` under the same configuration predicate.  The defining
implementation in `ipc/sem.c` owns the concrete `sem_undo_list` layout and
all operational behavior: `get_undo_list()` allocates it and installs the
pointer; `copy_semundo()` may increment its refcount and share that pointer;
and `exit_sem()` clears the task field before its refcounted release path.
`kernel/kcmp.c` also observes pointer identity without dereferencing it.

## Review dispositions

### Parity P1 — SPDX identifier

**Resolved.** The candidate's first line used `GPL-2.0-only`, whereas the
pinned header uses `GPL-2.0`.  The Rust provenance line now retains the exact
upstream SPDX identifier: `// SPDX-License-Identifier: GPL-2.0`.  No branding
allowlist entry authorizes this difference.

### Parity aggregate/ABI assessment

**Confirmed.** `#[repr(C)] pub struct sysv_sem` contains exactly one
`*mut sem_undo_list` member in source order.  A raw mutable pointer preserves
the C field's pointer-sized/aligned nullable, non-owning, and aliased
representation on each selected 64-bit target.  The named pointee is an
opaque, private-field zero-sized declaration because this header supplies only
a forward declaration and never embeds it by value.  No concrete pointee
layout is asserted or imported at this header boundary.

### Rust review — no candidate defect

**Confirmed.** The mapping introduces no Rust reference, ownership transfer,
`Drop`, allocation, lock, RCU action, refcount action, or `unsafe` boundary.
It therefore neither claims exclusive access nor changes the implementation's
allocation/sharing/destruction timing.  The raw pointer remains the correct
representation for the header-level Linux contract.

## Closed task semantic facts

- For both architectures, the include guard is preprocessing-only and carries
  no runtime, linkage, layout, ownership, or ABI payload in the Rust mapping.
- For both architectures, the `CONFIG_SYSVIPC` branch is selected because the
  frozen configurations set it to `y`; the sole member is consequently present
  in the translated aggregate.  There is no selected false branch to encode in
  this fixed configuration union.
- `struct sysv_sem` is a C-layout, one-field aggregate.  Its ABI is one
  pointer-sized, pointer-aligned `struct sem_undo_list *` field, with no
  export, callable symbol, padding-sensitive additional field, or by-value
  `sem_undo_list` layout in this header.
- `undo_list` is nullable and non-owning in this header.  Its storage, object
  lifetime, sharing/refcounting, locking/RCU, and teardown are all defined by
  the pinned `ipc/sem.c` implementation cited above; this header performs none
  of those operations.
- The forward-declared pointee intentionally remains opaque here.  Its
  concrete refcount/spinlock/list layout belongs to `ipc/sem.c` and is not an
  ABI claim made by `sem_types.h`.

These source facts close all task-local `PENDING_REVIEW` entries in
`SYMBOLS.tsv`, `ABI.tsv`, and `LIFETIMES.tsv` as recorded semantic evidence for
S014944; no `DRIVER_ABI.tsv` or `BLOCKERS.tsv` row is owned by this header.

The frozen queue fingerprint is
`af93adda6e7845ec178dc63a9462f88384392f78353165ea5a583ef78fcf423c`.
No compiler, formatter, rust-analyzer, linker, build, test, debugger, or
runtime tool was used.  This is a source-translation pipeline disposition
only.

## Final disposition

Both review reports are resolved.  S014944 is eligible for the atomic
`APPLYING` to `DONE` transition.
