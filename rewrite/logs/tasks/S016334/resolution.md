# S016334 applier resolution — BLOCKED

Task `S016334`, attempt `1`, pipeline `P02` was adjudicated from the pinned
`vendor/linux/include/uapi/linux/posix_acl.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the frozen task records, the sealed
candidate proposal, and both independent reviews.  No compiler, formatter,
analyzer, test, runtime tool, or historical Lupos source was used.

## Parity finding P1: include guard

**Disposition: unresolved — BLOCKED.**

The pinned header has the selected `#ifndef __UAPI_POSIX_ACL_H` at line 18,
the selected `#define __UAPI_POSIX_ACL_H` at line 19, and the selected matching
`#endif` at line 40.  `rewrite/SYMBOLS.tsv` records all three for both frozen
architectures, with the guard macro and both conditionals still
`PENDING_REVIEW`.  The candidate only asserts that a Rust module boundary is an
analogue; neither the pinned source nor any frozen scope/symbol/ABI/lifetime
record supplies a Rust module-loading or consumer-boundary mapping that proves
the C preprocessing behavior is preserved.  I cannot invent such a mapping.

## Parity finding P1: upstream copyright notices

**Disposition: correction required, but not applied.**

The pinned header retains the Andreas Gruenbacher (2002) and Red Hat, Inc.
(2016) notices at lines 2–15.  The candidate retains only the SPDX identifier,
so the notices are missing.  Adding them would alter the sealed candidate and
requires a fresh candidate snapshot and independent review; it cannot repair
the unresolved guard mapping above.

## Parity finding P1: stale candidate snapshot

**Disposition: correction required, but not applied.**

`candidate.diff` ends after `ACL_EXECUTE`, while the sealed source contains an
additional explanatory block before the constants.  Its bound digest
(`c6379e...5253e8`) therefore describes a different source snapshot.  Refreshing
it would change the semantic proposal binding and invalidate both review
attestations, so it cannot be performed during this sealed applier stage.

## Rust review

**Disposition: accepted for the twelve numeric constants only.**

The pinned header's twelve numeric macro values are represented as `i32`
constants with matching values, and there are no layout, ownership, or unsafe
constructs to resolve.  This does not establish the selected guard behavior or
cure the provenance and snapshot findings.

Because exact parity for the selected guard cannot be established from the
frozen source and records, this attempt is BLOCKED.  The destination source and
sealed artifacts were not edited.
