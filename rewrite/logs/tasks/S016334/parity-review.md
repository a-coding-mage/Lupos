# Parity review — S016334 (slot 1)

Reviewed independently against pinned `vendor/linux/include/uapi/linux/posix_acl.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the candidate
`src/include/uapi/linux/posix_acl.rs`, the supplied candidate snapshot, and frozen
scope/symbol/ABI/lifetime records.  No compiler, formatter, test, analyzer, or
historical Lupos source was used.

## Findings

### P1 — selected `__UAPI_POSIX_ACL_H` operative macro has no source-proven mapping

`vendor/linux/include/uapi/linux/posix_acl.h:18-19,40` contains the selected
conditional branch and defines `__UAPI_POSIX_ACL_H`.  `SYMBOLS.tsv` records that
guard and both conditional endpoints for x86_64 and aarch64 as selected operative
items.  The candidate instead says the guard “has no Rust analogue” and supplies no
defined interface or frozen source evidence showing that Rust module loading is the
required equivalent at every consumer boundary.  It therefore does not establish
the selected macro/conditional behavior required by the frozen inventory.

### P1 — relevant upstream copyright notices were dropped

The Linux header retains `Copyright (C) 2002 Andreas Gruenbacher` and `Copyright
(C) 2016 Red Hat, Inc.` in addition to its SPDX line (`posix_acl.h:2-15`).  The
candidate retains only SPDX.  The rewrite protocol requires retaining relevant
upstream copyright notices; this is an unauthorized provenance difference.

### P1 — `candidate.diff` is not a snapshot of the reviewed candidate

The supplied `candidate.diff` ends immediately after `ACL_EXECUTE`, while the
reviewed destination also contains a new explanatory block before the constants.
Consequently the required candidate artifact does not represent the source being
reviewed.  The snapshot must be refreshed through the prescribed implementation
evidence path before an applier can bind source-level review and semantic closure to
one candidate.

## Items confirmed

All twelve value macros from `posix_acl.h:21,24-25,28-33,36-38` are present under
their original names with the original numeric values.  The candidate’s `i32` type
matches the type of these unsuffixed C integer literals on the approved x86_64 and
AArch64 targets.  This header declares no structures, unions, functions, linkage,
or layout records, consistent with the empty S016334 ABI and lifetime rows.

## Verdict

FINDINGS.  The selected guard behavior and provenance/snapshot defects must be
resolved from frozen local source evidence; no parity approval is justified.
