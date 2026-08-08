# S016099 adjudication — not ready for closure

Scope: `include/uapi/linux/dev_energymodel.h` at pinned revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`, current candidate
`src/include/uapi/linux/dev_energymodel.rs`, and P02/a1 review evidence.  This
was source inspection only.  No compiler, formatter, linker, test, runtime,
or historical Lupos source was used.

## Evidence reopened

- The complete upstream header defines the two named C enum tags at lines
  19--21 and 33--37.  Its enumerators have enclosing C identifier scope; the
  domain values are `1`, `2`, and `4`.
- The pinned YNL specification classifies both definitions as `flags` and
  transports both as `u64` bitmasks.  The selected implementation likewise
  puts `pd->flags` into a `u64` netlink attribute.  The underlying energy-model
  implementation ORs the corresponding bits (`energy_model.c:527,666--668`).
- The two upstream macros are C string literals at header lines 10 and 80.
  The selected generated consumer initializes `genl_family.name` from the
  family macro (`em_netlink_autogen.c:51--53`); both relevant generic-netlink
  name fields are `char name[GENL_NAMSIZ]` (`include/net/genetlink.h:29--31,
  78--81`), and `GENL_NAMSIZ` is 16 (`include/uapi/linux/genetlink.h:8`).
- `rg` over the pinned tree finds the two C enum-tag spellings only in this
  UAPI header.  No pinned declaration or selected consumer supplies a layout,
  alignment, or calling-boundary use of either enum tag.

## Finding dispositions

### P1_ENUM_SCOPE_AND_FLAGS (parity review) — accepted

The current Rust fieldless enums change the C enumerator namespace and make a
closed Rust value domain where the source defines bit flags.  The missing
module-level identifiers and the non-composable value model are both parity
defects.  The correction is source-specified: the original enumerator names
must be public at their original scope and permit the source flag values,
their OR-combinations, and protocol extension bits.  A nominal Rust type, if
one remains, may not replace or narrow those public integer/bitmask values.

### RUST-002 (Rust review) — accepted

This is the same defect independently identified from Rust semantics.  The
YNL `type: flags` / `u64` declarations and the upstream OR operations confirm
that a closed fieldless Rust enum is not an equivalent substitute.  It has the
same controlled source-correction requirement as `P1_ENUM_SCOPE_AND_FLAGS`.

### P1_STRING_MACRO_C_ABI (parity review) — accepted

`&str` neither includes the C terminator nor carries C literal/fixed-array
initializer semantics.  The required source correction is exact: retain the
original names and literal bytes plus one terminating NUL (`"dev-energymodel\0"`
and `"event\0"`), with a representation usable for the fixed `char[16]`
generic-netlink initialization contract.  The correction must preserve C
aggregate-initializer zero-fill where the source destination exceeds the
literal length; a Rust fat string reference is not an acceptable substitute.

### RUST-001 (Rust review) — accepted

This is the same C-string representation defect independently identified from
Rust FFI semantics.  It has the same controlled source-correction requirement
as `P1_STRING_MACRO_C_ABI`.

## Frozen enum-ABI records and final disposition

The proposal rows for `ABI.tsv:190230` and `ABI.tsv:190231` leave
`alignment`, `export_kind`, and `layout` as `SOURCE_REVIEWED_VALUE` for the two
named C enum tags.  That phrase only records that source was inspected; it is
not an exact target-ABI value.  The pinned header establishes names and
discriminators, the selected source establishes flag payload behavior, and the
frozen aarch64 context identifies the target.  None establishes the C enum
object layout/alignment or a Rust representation that has that exact ABI.

Consequently, a controlled requeue is source-backed for the two accepted
implementation defects, but it cannot close this task: the enum ABI records
remain unresolved after those corrections.  No frozen local layout evidence
was found, and Phase 1 forbids producing compiler evidence.  Under the
source-only rule, the required final disposition is **BLOCKED**, not `DONE`.

To resume, obtain auditable Phase 0 target-ABI evidence for both enum tags
(including representation, size, and alignment under the frozen aarch64
compiler/configuration), bind it to the existing Phase 0 identity, then
perform the controlled source requeue and fresh independent reviews.  Do not
infer `i32`, reuse the current `#[repr(i32)]` enums, or treat the absence of a
current in-tree by-value use as proof of the UAPI type ABI.

No source, queue, or closure-manifest mutation was performed by this
adjudication.
