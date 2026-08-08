# S012620 application resolution — attempt 4

## Outcome

**NOT READY FOR `DONE`; recommend `BLOCKED` pending a controlled Phase 0
inventory/scope correction.**  Per the directed application scope, this
resolution changes neither `src/include/crypto/dh.rs` nor the queue.

I reopened the complete pinned `vendor/linux/include/crypto/dh.h` and
`vendor/linux/crypto/dh_helper.c` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the selected AArch64
configuration (`CONFIG_CRYPTO_DH=m`), Kbuild ownership
(`crypto/Makefile:40-42`), current candidate, both review reports, and the
exact frozen S012620 rows.

## Finding F1 — upstream SPDX identifier differs

**Disposition: ACCEPTED; candidate correction required.**

`vendor/linux/include/crypto/dh.h:1` is exactly
`SPDX-License-Identifier: GPL-2.0-or-later`.  The current candidate instead
states `GPL-2.0-only`, and `rewrite/BRANDING_ALLOWLIST.tsv` has no licence
identifier allowance.  This is not an allowed branding delta and fails the
required retention of the upstream SPDX identifier.

Required remediation after the blocked inventory issue is resolved: change
only the candidate's SPDX line to `GPL-2.0-or-later`, recreate candidate-bound
evidence, and obtain fresh independent reviews before application resumes.  I
made no source edit in this application pass.

## Finding F2 — four header interfaces absent from frozen task records

**Disposition: ACCEPTED; Phase 1 cannot close the task from the frozen
records.**

The pinned selected header declares these externally linked interfaces:

- `crypto_dh_key_len` at `include/crypto/dh.h:51`;
- `crypto_dh_encode_key` at `include/crypto/dh.h:66`;
- `crypto_dh_decode_key` at `include/crypto/dh.h:80`;
- `__crypto_dh_decode_key` at `include/crypto/dh.h:95-96`.

Their exact signatures are corroborated by definitions in
`crypto/dh_helper.c:34`, `:40`, `:66`, and `:94`; the first, second, and
fourth named public helpers are GPL-exported at `:38`, `:64`, and `:120`,
while the double-underscore helper is externally linked but has no export
macro.  The candidate's `unsafe extern "C"` declarations preserve the names,
C ABI, pointer constness/mutability, and `unsigned int`/`int` widths.  The
Rust reviewer correctly found no Rust-source defect in those declarations.

Nevertheless, frozen `SYMBOLS.tsv` rows 143217-143220 contain only the guard
records and `struct dh`; frozen `ABI.tsv` row 94741 and `LIFETIMES.tsv` row
90682 contain only `struct dh`.  None records the four selected declarations'
symbol mapping, linkage/export status, calling convention, error contract, or
raw-buffer lifetime/aliasing contract.  Existing semantic-closure proposal
records can close only those pre-existing rows; they cannot create the missing
source inventory.  The candidate cannot cure this Phase 0 omission.

This is not a semantic dispute: `crypto/dh_helper.c:40-62` establishes encode
buffer behavior and `:66-120` establishes that decode writes `struct dh` and
sets all three pointees into the input packet buffer without allocation, with
the public decoder adding validation.  It does establish the information
needed for authoritative records, but the protocol requires those records to
be complete before this task is `DONE` and forbids silently discovering or
adding work in Phase 1.

Required remediation: reopen the Phase 0 inventory/scope gate under the
authorized workflow; add and review authoritative `SYMBOLS.tsv`, `ABI.tsv`,
and `LIFETIMES.tsv` records for all four declarations; regenerate every seal
and queue binding that the correction invalidates; then requeue S012620 for a
fresh candidate/review/application attempt.  Until that controlled correction
exists, the queue must not transition to `DONE`; the prescribed disposition is
`BLOCKED`, not a manual manifest edit or an unreviewed candidate acceptance.

## Existing-record closure

The existing S012620 semantic proposal is source-supported only for its
pre-existing guard and `struct dh` rows.  `struct dh` retains the C order of
three `const void *` fields followed by three `unsigned int` fields
(`include/crypto/dh.h:32-39`), is a non-owning aggregate, and its decoded
pointees borrow the packet buffer (`crypto/dh_helper.c:84-88`).  This does not
resolve F2 because no frozen rows exist for the declarations themselves.

No compiler, formatter, linker, runtime, test, benchmark, or historical Rust
source was used.
