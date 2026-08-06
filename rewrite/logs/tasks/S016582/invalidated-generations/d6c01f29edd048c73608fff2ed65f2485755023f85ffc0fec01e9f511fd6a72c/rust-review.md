# Rust review — S016582

## Verdict

Reject pending correction of the enumerator namespace and closure of the
recorded ABI decision.  No compiler, formatter, test, or runtime command was
run.

## Finding R1 — C enumerators were moved into a different namespace (major)

`include/xen/interface/io/xenbus.h:19-39` declares each `XenbusState*` name as
an enumerator.  These are ordinary file-scope identifiers for every C
translation unit including the header; the selected dependent API header uses
them as bare expressions (for example, `include/xen/xenbus.h` declares
`enum xenbus_state` fields and APIs, while Xenbus consumers pass and compare
bare `XenbusState*` values).

The candidate instead defines only associated constants at
`src/include/xen/interface/io/xenbus.rs:20-29`.  That changes every name from
`XenbusStateConnected` to `xenbus_state::XenbusStateConnected`, so the source
enumerator interface is absent.  Define the nine `pub const XenbusState*:
xenbus_state = xenbus_state(<same value>);` items at module scope (or provide
an equivalent mapping that preserves the original exported identifier names).
Associated aliases may remain only if they do not replace those identifiers.

## Finding R2 — `i32` enum ABI assertion still lacks frozen evidence (must
close before DONE)

The transparent `xenbus_state(pub i32)` representation is a reasonable
candidate: the selected AArch64 compile command for
`arch/arm/xen/enlighten.c` has no `-fshort-enums`, and the nine source values
are `0..8`.  It is not, however, sufficient evidence that the pinned
Clang/AArch64 C `enum xenbus_state` has the signed-`int` ABI assumed by the
candidate.  The authoritative `rewrite/ABI.tsv` row for this exact type is
still `PENDING_REVIEW` (and the matching lifetime row is also pending).

The applier must record the pinned-source/toolchain-derived size, alignment,
integer representation, and parameter/field ABI decision before marking this
task `DONE`.  If that decision is C `int`, `#[repr(transparent)]` over `i32`
preserves the required one-field representation; it must not be accepted only
because the named values themselves fit in `i32`.

## Checked items

- Provenance exactly matches `vendor/linux.SHA`
  (`425f94c2954b1fe80ebdbf9b29854e89750355df`), path, task ID, and AArch64
  scope.
- All nine upstream values and spellings occur once in the candidate; no
  conditional source branch has runtime content to port.
- `#[repr(transparent)]` gives the wrapper the inner scalar layout once R2 is
  established; the public field preserves raw representability rather than
  incorrectly making a closed Rust enum.
- No `unsafe`, `unsafe fn`, `unwrap`, `expect`, panic macro, placeholder,
  Rust test configuration, unauthorized branding, or project-authored test is
  present.
