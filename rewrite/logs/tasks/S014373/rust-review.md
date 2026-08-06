# Rust review — S014373

Reviewed only the fresh candidate `src/include/linux/migrate_mode.rs` against
the pinned `vendor/linux/include/linux/migrate_mode.h` and direct pinned caller
context.  No compiler, formatter, test, runtime, historical Rust source, or
diagnostic was used.

## Findings

### R1 — High: fieldless Rust enums impose an invalid-discriminant invariant not present at the C boundary

`migrate_mode` is carried over C callback boundaries: for example,
`struct movable_operations::migrate_page` in `include/linux/migrate.h` and
`struct address_space_operations::migrate_folio` in `include/linux/fs.h` take
it by value.  It is also stored in trace-event fields.  The retained original
driver/object side can therefore supply the C object representation directly.

The candidate's `#[repr(C)]` fieldless enums permit only their named Rust
variants as valid values.  A value received through C or raw storage whose
integer representation is not one of those variants cannot safely be treated
as this Rust enum; merely forming/reading such an enum creates Rust validity
requirements that the C declaration does not establish.  The C header has no
run-time validation boundary, and its enum-compatible integer representation
can be passed through the listed function-pointer ABI.

Resolve this by using an ABI-proven integer-backed representation that can
carry every C-representable value (normally a transparent newtype plus named
constants), or by establishing and enforcing a validated conversion boundary
before any Rust enum is formed.  The latter must cover every C/driver callback
and raw-storage ingress.  Do not rely on `#[repr(C)]` alone to make arbitrary
C enum bit patterns valid Rust variants.

### R2 — High: the frozen C enum ABI is still unproven for both architectures

All four `ABI.tsv` records for `enum migrate_mode` and `enum migrate_reason`
remain `PENDING_REVIEW` for x86_64 and aarch64.  They do not state the C enum's
size, alignment, compatible integer type/signedness, or its frozen compiler
flags.  `#[repr(C)]` asks Rust to use a target C-enum representation, but the
candidate and its evidence do not close the required ABI record or demonstrate
that this agrees with the pinned LLVM invocation for both selected targets.

This is material because `enum migrate_mode` appears by value in callback
function-pointer ABI and trace data, while `enum migrate_reason` is stored in
`struct migration_target_control` (`mm/internal.h`).  Before application,
derive the exact representation from the pinned Phase 0 target command and
record size, alignment, signedness, and C/Rust ABI use for both targets.  The
chosen Rust representation must then encode that result explicitly enough to
preserve both layout and call ABI.  If that exact evidence cannot be
established, this task must be blocked rather than accepted.

### R3 — Medium: C's freely copyable/comparable value semantics were not carried into Rust

Neither enum derives `Copy`, `Clone`, `PartialEq`, or `Eq`.  The pinned C code
passes `mode` by value and subsequently compares it repeatedly (for example
`mm/migrate.c:migrate_pages_batch` and `mm/compaction.c:update_cached_migrate`)
without a move operation.  A Rust fieldless enum without `Copy` instead moves
on a by-value transfer; it also lacks the direct equality operation required
for the corresponding comparisons.  Later direct translations would be forced
to introduce borrow/match workarounds or fail to retain the C value semantics.

Whichever representation resolves R1/R2 should be explicitly `Copy` and
`Clone`, and provide equality semantics matching integer comparison (normally
`PartialEq, Eq`).  These derives must not be used to mask the invalid-value
issue in R1.

## No finding

The candidate has the required immutable provenance, maps every named
enumerator in source order with the correct zero-based values, introduces no
padding-bearing fields or niche-based option representation, and contains no
unsafe code.  `MR_TYPES` remains a value/sentinel rather than a Rust
variant-count substitute.  The SPDX/provenance form matches the project
mandated source-header template.

## Review disposition

Changes are required before this candidate can be accepted.  The applier must
also close the task's `PENDING_REVIEW` ABI and lifetime records for both frozen
architectures with source-grounded evidence.
