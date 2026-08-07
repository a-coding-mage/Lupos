# Parity review — S016267, slot 1

Verdict: FINDINGS

Reviewed the sealed attempt-1 candidate for `include/uapi/linux/netdev.h` against
the pinned source at `425f94c2954b1fe80ebdbf9b29854e89750355df`. Candidate
binding: `f24cb1108af94200ee7a600fac46c70a86ad7f5182fa08b8da014df602b81fd2`.
The sealed proposal contains 921 records and is bound to queue fingerprint
`cfa8dcf0d81c408de38d4202fe1ce9d9e5e301d1e67439c9530da94b1e64ee3f` and
Phase 0 identity `0123af9e96b58af4e98b91a2de5608d36b63c67864b9041d3340587f3dbe40d2`.

The six named enum aliases, all anonymous-enum attribute and command constants,
sparse initializers, derived maxima, numeric values, names, and common-source
provenance otherwise correspond to the pinned header. There are no source
conditionals beyond the header guard.

## Findings

### P01-S016267-PARITY-1 — string macro representation is not C-UAPI equivalent

Pinned source lines 10, 249, and 250 define `NETDEV_FAMILY_NAME`,
`NETDEV_MCGRP_MGMT`, and `NETDEV_MCGRP_PAGE_POOL` as C string-literal macros.
Each expansion designates a NUL-terminated character array suitable for the C
netlink-family/group interface. The candidate instead exports Rust `&str`
values at lines 7, 172, and 173. A Rust `&str` is a fat slice and supplies no
trailing NUL or C character-array/pointer representation, so it cannot preserve
that UAPI contract.

Affected proposal records:

- `SC1-2b0780cf961cecf9f362d8a378aede1a07382de8bb261f6f815e61a06dc46d36`
- `SC1-f2ec8c26ac9a162087526007207ae1c58d9ef5531bfa6235b4d4a6190cf4bb48`
- `SC1-70ffab2cdccc6c3f2ac8d0c06a91002ca0105404b0941de49f1be7a41bb6059c`
- `SC1-7582c7ee037cf90768992d806553d15729abea023d4e22178632fce533045dfd`
- `SC1-1c250d20e165b164dd7cf9ef6dae8c63f66c69e008f3b5aa29a4243cd7e8913a`
- `SC1-496721e28a8ebc548130ec415e8a41d3ac9badcda5a0ab48417d56b54246c6ae`

Resolution required: represent these literal macro values with a C-compatible,
NUL-terminated byte/string form while retaining their exact spelling.

### P01-S016267-PARITY-2 — include-guard macro omitted while recorded as complete

Pinned source lines 7–8 and 252 define and use `_UAPI_LINUX_NETDEV_H` to make
the UAPI header’s inclusion semantics explicit. The candidate has no equivalent
item or documented Rust-module mapping for this selected operative macro, while
the sealed proposal marks its selection-expression records complete. This leaves
the guard’s treatment unestablished rather than faithfully mapped.

Affected proposal records:

- `SC1-a9213bd13f1dcf4b0a370d27d98378c9295d2e69f0405408b8e1fee3713efd67`
- `SC1-9a2423ccf5e2f14e227629fb1c456fada44ba65ea5e9dc0896f18aba0d6ae212`

Resolution required: provide and evidence the exact Rust-side mapping for the
single-inclusion guard, or revise the closure record through the authorized
applier path to state why Rust module semantics are the faithful equivalent.
