# Parity review — S016582 / attempt 1

Reviewer: parity_reviewer (`gpt-5.6-terra`, high)  
Scope: `vendor/linux/include/xen/interface/io/xenbus.h` →
`src/include/xen/interface/io/xenbus.rs` (aarch64)

## Result

FINDINGS — the candidate is not source-proven equivalent to the selected Xen
header and cannot be accepted as a UAPI/core ABI translation.

### P1 — `enum xenbus_state`: Rust `#[repr(C)]` enum does not establish the selected C ABI

Linux defines the public C `enum xenbus_state` at
`include/xen/interface/io/xenbus.h:17-39`.  The frozen ABI record for this
exact type (`rewrite/ABI.tsv`, task `S016582`) records both the ABI form and
layout as `PENDING_REVIEW`; it supplies no source-proven aarch64 Rust mapping.
The candidate substitutes a Rust fieldless `#[repr(C)] enum` and claims that
it “preserves the integer enum ABI,” but provides neither a fixed integer
representation nor an ABI wrapper/mapping that proves its size, alignment,
calling convention, or field layout equals the pinned C enum on aarch64.

This type is not merely internal: pinned
`include/xen/xenbus.h:87` embeds it in `struct xenbus_device`, and lines
`117`, `219`, `232`, and `240` use it in callback/function interfaces.  The
candidate therefore changes an unresolved ABI-bearing public type without the
required source-level ABI evidence.

### P1 — `enum xenbus_state`: the candidate narrows the C value domain at an unvalidated Xen-store ingress

Pinned `drivers/xen/xenbus/xenbus_client.c:945-959` declares
`enum xenbus_state result` and passes `&result` to
`xenbus_gather(..., "state", "%d", &result, NULL)` before returning it.
The code validates only whether `xenbus_gather` returned an error; it does not
validate that the stored integer is one of `0..8`.  Likewise,
`xenbus_strstate` at lines `97-111` explicitly has an out-of-range path
(`"INVALID"`).  A C enum object is the unvalidated integer storage passed by
that interface; the selected source does not constrain all values to the nine
enumerators.

The Rust `enum xenbus_state` admits only its nine declared discriminants as a
valid Rust value.  No raw integer storage, checked conversion, or exact
invalid/out-of-range behavior is provided, so the candidate cannot represent
the pinned ingress and subsequent public interfaces without introducing an
invalid Rust enum value or changing behavior.

### P1 — `XenbusState*` enumerators: C ordinary-identifier namespace is not preserved

The header’s enumerators at `xenbus.h:19-38` are C enumerator constants in the
including translation unit’s ordinary identifier namespace.  Pinned consumers
use them unqualified, for example the designated initializers at
`drivers/xen/xenbus/xenbus_client.c:100-108` and the comparison at line `246`.
The candidate makes them variants that require the Rust type namespace
(`xenbus_state::XenbusStateUnknown`, etc.) and exports no same-name constants.
That changes the selected header’s symbols/namespace and does not provide a
mapping for every selected `enum_constant` record in `rewrite/SYMBOLS.tsv`.

### P1 — `_XEN_PUBLIC_IO_XENBUS_H`: selected include guard has no equivalent mapping

`rewrite/SYMBOLS.tsv` selects the `_XEN_PUBLIC_IO_XENBUS_H` operative macro
and its `#ifndef`/`#endif` branch for this task.  The candidate contains no
module/include mapping or exported compatibility guard for that selected
preprocessor contract.  As this Xen interface is consumed through the pinned
C-facing header graph (`rewrite/metadata/header_closure.tsv` reports 31
consumers), omission cannot be treated as a comment-only change.

## Source inspected

- `vendor/linux/include/xen/interface/io/xenbus.h`
- Candidate snapshot `rewrite/logs/tasks/S016582/candidate.diff`
- Frozen task records in `rewrite/SCOPE.tsv`, `rewrite/SYMBOLS.tsv`,
  `rewrite/ABI.tsv`, `rewrite/LIFETIMES.tsv`, and
  `rewrite/metadata/header_closure.tsv`
- Pinned direct consumer declarations in `vendor/linux/include/xen/xenbus.h`
  and `vendor/linux/drivers/xen/xenbus/xenbus_client.c`

No compiler, formatter, test, analyzer, or historical Lupos source was used.
