# Parity review — S014944 / slot 1

## Scope and method

Source-only comparison of `src/include/linux/sem_types.rs` with the complete
pinned `vendor/linux/include/linux/sem_types.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.  Verified the task row is
`REVIEWING`, leased to P01, and maps that source to that destination for the
common architecture set.  Consulted the frozen x86_64 and AArch64 configs,
scope/symbol/ABI/lifetime records, `include/linux/sched.h`, and the relevant
`ipc/sem.c` ownership/use sites.  No compiler, formatter, analyzer, build, or
test was run.

Both frozen configurations set `CONFIG_SYSVIPC=y`.  Thus the selected C layout
is exactly one `struct sem_undo_list *undo_list` field.  The candidate's
`#[repr(C)] sysv_sem` has one mutable pointer field of the named opaque type;
on both approved architectures this preserves the selected field presence,
pointer-sized layout/alignment, and nullability.  The opaque zero-sized Rust
declaration is used only behind that pointer and does not expose an alternate
payload layout in this header.  This agrees with the upstream forward
declaration and with `ipc/sem.c`, which owns the pointed-to object, refcount,
lock, RCU list, clone sharing, and exit teardown.  Provenance source path,
revision, architecture set, and task ID all match the frozen row.

## Finding

### P1 — SPDX identifier was changed

- **Candidate:** `src/include/linux/sem_types.rs:1` says
  `SPDX-License-Identifier: GPL-2.0-only`.
- **Pinned source:** `include/linux/sem_types.h:1` says
  `SPDX-License-Identifier: GPL-2.0`.
- **Impact:** The translation changes the retained upstream SPDX identifier.
  The rewrite rules require relevant upstream SPDX identifiers to be retained;
  this is outside the branding allowlist and is not a permitted semantic or
  provenance delta.
- **Required resolution:** Preserve the pinned file's exact SPDX identifier in
  the Rust file.

## Result

One finding.  Other than the SPDX discrepancy, the selected `CONFIG_SYSVIPC`
field, forward-pointer ABI, and provenance are source-parity matched.  This is
a source-review result only; it makes no compile, link, runtime, or test claim.
