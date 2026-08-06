# Resolution — S014373

## Disposition: BLOCKED

The pinned source establishes both enum declaration order and their named
values, but does not establish their object representation. The frozen,
identity-bound ABI evidence is incomplete: all four records for `enum
migrate_mode` and `enum migrate_reason` (x86_64 and aarch64) in
`rewrite/ABI.tsv` retain `layout=PENDING_REVIEW`,
`alignment=PENDING_REVIEW`, and `status=PENDING_REVIEW`. The matching four
lifetime records remain `PENDING_REVIEW` in `rewrite/LIFETIMES.tsv`. The Phase
0 identity pins Clang 19 and both configurations, but it supplies no accepted
per-target C-enum size, alignment, signedness/compatible integer type, or ABI
measurement for these declarations. This applier may not assume that
`#[repr(C)]`, `i32`, or any other Rust representation closes that missing
evidence.

This ABI is material, not merely descriptive. Pinned
`include/linux/migrate.h:44-49` and `include/linux/fs.h:426-430` carry
`enum migrate_mode` by value through callback signatures; `mm/internal.h:1041`
stores it in `struct compact_control`; and `mm/internal.h:1520` stores
`enum migrate_reason` in `struct migration_target_control`. Pinned
`include/linux/migrate.h:59-61` accepts a migration reason as `int`, while
`mm/migrate.c:1354-1357` passes that value to an `enum migrate_reason`
parameter. The original driver/object side can therefore provide C storage or
callback arguments without a Rust closed-enum validity guarantee.

### P1 / R1 / R3 — closed Rust enum and C value semantics: unresolved

The existing fieldless Rust enums are not accepted. Their validity set is only
the named Rust variants, whereas the C declarations and the cited raw-storage,
field, and callback paths establish no validated conversion boundary. `Copy`,
`Clone`, and equality derives would address only ordinary C copy/compare use;
they would not make arbitrary C integer object values valid Rust enum values.
A transparent integer representation with named constants is only acceptable
after the frozen C ABI establishes its exact underlying representation for both
targets. No such representation was invented here.

### R2 — exact C enum ABI: unresolved and blocking

The required source-grounded Phase 0 ABI record must state, for each selected
target and each enum, the C size, alignment, compatible integer type and
signedness, relevant frozen command flags, and the resulting field and
by-value callback ABI. Until that identity-bound evidence exists, the exact
Rust representation cannot be selected. The task must remain blocked rather
than assume `#[repr(C)]` or an integer width.

### P2 — upstream migration blocking invariants: resolved in the candidate

The candidate documentation now retains the source invariants from
`include/linux/migrate_mode.h:4-10`: `MIGRATE_ASYNC` never blocks;
`MIGRATE_SYNC_LIGHT` may block for most operations but not `->writepage`
because of significant potential stall time; and `MIGRATE_SYNC` blocks during
page migration. This also preserves the callback requirement in
`include/linux/fs.h:426-430` that an asynchronous migrate-folio operation must
not block.

### P3 — SPDX: resolved in the candidate

The candidate SPDX identifier is restored exactly to the pinned header's
`GPL-2.0` identifier.

No compiler, formatter, linker, test, runtime command, diagnostic, historical
Rust source, or non-task source file was used in this source-only application.
