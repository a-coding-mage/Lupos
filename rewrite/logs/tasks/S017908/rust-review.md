# Rust review — S017908

Reviewer: `rust_reviewer` (`gpt-5.6-terra`, `high`)

Scope reviewed: `src/net/ipv6/ip6_offload_h.rs` against pinned
`vendor/linux/net/ipv6/ip6_offload.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

## Result

No Rust-source findings.

## Source-level checks

- Provenance is exact: the candidate identifies the task, Linux path, frozen
  revision, and `common` architecture membership
  (`src/net/ipv6/ip6_offload_h.rs:1-5`); these agree with the S017908 queue
  row and `vendor/linux.SHA`.
- The complete upstream header contains precisely four external, no-argument
  declarations returning C `int` (`vendor/linux/net/ipv6/ip6_offload.h:10-13`).
  The candidate contains exactly those four identifiers with no extra
  declarations (`src/net/ipv6/ip6_offload_h.rs:11-16`).
- `core::ffi::c_int` represents the declared C `int`, and `extern "C"`
  preserves the C calling convention and unmangled default link names for
  `ipv6_exthdrs_offload_init`, `udpv6_offload_init`,
  `udpv6_offload_exit`, and `tcpv6_offload_init`
  (`src/net/ipv6/ip6_offload_h.rs:9-16`).  The definitions confirm these are
  external `int` functions (`vendor/linux/net/ipv6/exthdrs_offload.c:23`,
  `vendor/linux/net/ipv6/udp_offload.c:191-205`, and
  `vendor/linux/net/ipv6/tcpv6_offload.c:194-203`).
- Retaining the declarations as foreign functions is appropriate for a header:
  the functions are implemented in their owning source translations.  The
  interface does not create Rust references, expose layouts, or perform casts,
  allocation, panics, or cleanup itself.
- Marking the foreign block `unsafe` keeps calls to lifecycle-sensitive global
  initialization/teardown explicit.  Upstream invokes the init operations in
  ordered network initialization paths and calls `udpv6_offload_exit` only on
  the corresponding unwind path (`vendor/linux/net/ipv6/af_inet6.c:1106-1114`,
  `vendor/linux/net/ipv6/af_inet6.c:1170-1178`); the declaration layer does not
  weaken that contract.  No foreign-unwind or panic path is introduced by this
  file.
- The upstream include guard is preprocessor-only
  (`vendor/linux/net/ipv6/ip6_offload.h:7-15`) and has no Rust ABI, ownership,
  or runtime analogue; its absence from the Rust module is not a semantic
  omission.

The six S017908 guard/macro records in `rewrite/SYMBOLS.tsv` remain marked
`PENDING_REVIEW`; this review resolves their source meaning as a
preprocessor-only include guard.  The applier must record the final manifest
closure required before `DONE`.
