# S012620 application resolution — attempt 1

- Task: `S012620`
- Pipeline: `P01`
- Stage at adjudication: `APPLYING`
- Linux source: `vendor/linux/include/crypto/dh.h`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Candidate reviewed: `src/include/crypto/dh.rs` (the candidate sealed by the
  current implementation and both current review reports)

## PARITY-COPYRIGHT-001 — accepted; correction required

`vendor/linux/include/crypto/dh.h:2-6` contains the relevant upstream notice:
`Copyright (c) 2016, Intel Corporation` and the Salvatore Benedetto authorship
line.  The present candidate retains only the SPDX line and immutable
provenance, so it does not retain that upstream copyright notice as required by
the source-tree rule.

The required candidate correction is limited to adding those two notice lines
as Rust comments, preserving the SPDX identifier and the immutable provenance.
It changes neither ABI nor behavior, but it is still a source change.  This
finding is therefore not dismissed and cannot be closed against the existing
candidate seal.

## RUST-COPY-SEMANTICS — accepted; correction required

`struct dh` at `vendor/linux/include/crypto/dh.h:32-39` is a plain aggregate
of three non-owning pointer values and three `unsigned int` values.  It has no
destructor or ownership transfer, so an ordinary C value copy is a shallow copy
of exactly those six fields.  The direct helper context confirms the pointers
are borrowed: on successful decode, `crypto/dh_helper.c:84-90` assigns `key`,
`p`, and `g` to locations in the caller's packet buffer without allocation.

The current `#[repr(C)] pub struct dh` has no `Drop`, but lacks `Copy` and
`Clone`; an ordinary Rust assignment consequently moves the source binding
rather than making the C-equivalent shallow aggregate copy.  The required
candidate correction is `#[derive(Copy, Clone)]` on `dh`.  It adds no fields,
layout change, destructor, allocation, aliasing abstraction, or lifetime claim;
it restores only the C aggregate's fieldwise-copy behavior.  This is a source
change and cannot be closed against the existing review seals.

## FFI, linkage, layout, and lifetime adjudication — no blocker

The header declarations are supported by the pinned in-scope implementation
owner, not an unproved external dependency.  `rewrite/SCOPE.tsv` assigns
`crypto/dh_helper.c` to S001192, a selected `RUST_TRANSLATE` dependency of this
task in the AArch64 `crypto/dh_generic.o` module.  Its frozen symbol records
and `vendor/linux/crypto/dh_helper.c:34-120` establish all four matching
external definitions: `crypto_dh_key_len`, `crypto_dh_encode_key`,
`__crypto_dh_decode_key`, and `crypto_dh_decode_key`; the first, second, and
fourth have `EXPORT_SYMBOL_GPL`, while the double-underscore helper is
externally declared but not exported.  A header declaration does not itself
alter that definition/export distinction.

The candidate's `unsafe extern "C"` declarations retain each source spelling,
the C calling boundary, raw-pointer constness/mutability, `unsigned int` as
`u32`, and `int` as `c_int`.  `#[repr(C)] dh` retains the source field order of
three `const void *` fields followed by the three `unsigned int` fields.  The
frozen AArch64 ABI proposal records natural pointer/u32 layout and 8-byte
aggregate alignment; the raw pointers preserve the caller-owned packet-buffer
lifetime without fabricating a Rust borrow.  No FFI, linkage, ABI, ownership,
locking, RCU, or refcount question remains that requires `BLOCKED` for this
header task.

## Required terminal disposition

Do **not** mark S012620 `DONE`.  The candidate and its implementation/candidate
evidence must be regenerated after exactly the two corrections above, then the
task must receive fresh independent parity and Rust reviews and a fresh
application.  The current reports remain evidence only for the current,
uncorrected candidate and must not be treated as seals for the corrected one.

Recommendation to the queue owner: perform the controlled requeue prescribed
by the queue workflow, preserving the frozen task ID, source path, destination
path, scope, and dependency set.  No queue mutation or source mutation was
performed by this adjudication.
