# Applier resolution — S016344

Task `S016344` remains at `APPLYING` for source-only adjudication.  This
record does not change the candidate, queue, frozen manifests, or task scope.

## Finding F1 — named `psp_version` enumerators lack ordinary-identifier mappings

**Disposition: accepted; source-proven correction exists, but not applied in
this adjudication pass.**

Pinned `include/uapi/linux/psp.h:13-18` declares the tag `enum psp_version`
and four enumerators.  The enumerators have the exact consecutive values
`0`, `1`, `2`, and `3`; `SYMBOLS.tsv` records each value for both approved
architectures.  The candidate supplies the Rust variants only, whereas the
same C enumerator spellings are used as unqualified integer expressions in
the pinned tree: `net/psp/psp_main.c:173-184` switches on a `u32`, and the
selected consumers use the constants in `case` labels and shifts.

A corrected candidate must therefore export module-level, same-spelled
`core::ffi::c_int` constants with values `0..=3`, in addition to preserving
the source-level `psp_version` tag mapping.  This is the same representation
already used for the header's anonymous enum identifiers and restores the
C ordinary-identifier surface without changing scope.

## Finding F2 — C string-literal macro mappings omit the NUL and array form

**Disposition: accepted; source-proven correction exists, but not applied in
this adjudication pass.**

Pinned `include/uapi/linux/psp.h:10,95-96` defines `PSP_FAMILY_NAME`,
`PSP_MCGRP_MGMT`, and `PSP_MCGRP_USE` as the literals `"psp"`, `"mgmt"`, and
`"use"`.  Their C literal arrays include terminal NUL bytes, respectively
`[b'p', b's', b'p', 0]`, `[b'm', b'g', b'm', b't', 0]`, and
`[b'u', b's', b'e', 0]`.  The current `&str` declarations omit those bytes
and instead create Rust fat slices.  The pinned family initializer at
`net/psp/psp-nl-gen.c:161-163` uses `PSP_FAMILY_NAME` to initialize
`struct genl_family.name`, declared `char name[GENL_NAMSIZ]` in
`include/net/genetlink.h:78-82`.

A corrected candidate must expose each same-spelled macro mapping as an
explicit fixed-size, NUL-terminated C-byte array representation (not `&str`),
with lengths 4, 5, and 4.  The correction must preserve only the literal
bytes and array semantics; it must not invent linkage or change the frozen
destination/scope.  It needs fresh independent review after application.

## Finding RUST-S016344-001 — Rust `&str` is not a C string-literal array

**Disposition: accepted; co-resolved by the required F2 correction, but not
applied in this adjudication pass.**

The same pinned declarations and family-initializer context establish the
reviewer's result.  Replacing each `&str` with the exact NUL-terminated fixed
array specified above resolves the fat-slice, missing-terminator, and
C-character-sequence differences.  No `unsafe`, allocation, or runtime
mechanism is justified by this header.

## Blocking semantic record — `enum psp_version` ABI

**Disposition: unresolved; recommend `BLOCKED`, not `DONE` or a source
requeue.**

The candidate claims that `#[repr(C)] pub enum psp_version` preserves the C
enum representation, but the pinned header only supplies the tag and four
enumerator values.  A complete pinned-tree search finds no use of the type
spelling outside its declaration; all direct uses are unqualified enumerator
integer expressions.  The frozen ABI records for the named enum remain
`PENDING_REVIEW` for both `aarch64` and `x86_64`, with layout and alignment
also `PENDING_REVIEW` (`rewrite/ABI.tsv`, S016344, source line 13).  The
corresponding lifetime records are likewise unresolved
(`rewrite/LIFETIMES.tsv`, S016344, source line 13).  No frozen compile-metadata
record establishes enum layout or an enum-size selection for this header.

Consequently the required representation, alignment, and external UAPI
context for the distinct `enum psp_version` type cannot be established from
the permitted pinned local source and frozen records.  The two concrete
corrections above are insufficient to close every semantic record.  Under the
Phase 1 protocol, the controlling outcome is `BLOCKED` with a reason naming
this unresolved enum ABI/context; a controlled source requeue would be
premature unless new permitted authoritative evidence resolves it.

No compiler, formatter, linker, test, runtime command, historical source, or
diagnostic was used for this adjudication.
