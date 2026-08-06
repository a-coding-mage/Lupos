# Parity review — S017908

Role: parity reviewer (slot 1)  
Pipeline: P01  
Reviewed candidate: `src/net/ipv6/ip6_offload_h.rs`  
Pinned source: `vendor/linux/net/ipv6/ip6_offload.h` at `425f94c2954b1fe80ebdbf9b29854e89750355df`

## Result

PASS — no parity findings.

## Source comparison

- The source header contains exactly four unconditioned, external declarations:
  `ipv6_exthdrs_offload_init`, `udpv6_offload_init`,
  `udpv6_offload_exit`, and `tcpv6_offload_init`.  The candidate declares all
  four once, with unchanged spelling, no parameters, and `c_int` results,
  matching C `int (void)` on both frozen architectures.
- The candidate's `extern "C"` declarations retain the C calling convention
  and external symbol names required by the source declarations.  The source
  declares no `static`, `const`, visibility, export, section, or function
  attributes in this header; no corresponding declaration-level qualifier is
  omitted.  The `__init` annotations on three definitions reside in their
  implementation files, not in this header.
- The include guard has no translated runtime/API member beyond preventing
  repeated declarations.  The source has no configuration branches, types,
  variables, macros other than that guard, inline bodies, cleanup logic, or
  additional declarations to translate.
- Direct source consumers and definition sites agree with these signatures:
  `af_inet6.c` consumes the UDP init/exit results and ordering, while
  `ip6_offload.c` consumes TCP and extension-header init results.  The
  corresponding definitions are global `int` functions; none is an
  `EXPORT_SYMBOL` API.
- Provenance is exact: SPDX identifier, Linux path, frozen revision, common
  architecture scope, and task ID all match the queue and pinned source.
  `CONFIG_IPV6=y` is selected in both frozen configurations.  No branding
  change or unselected conditional applies.

No source, build, formatter, analyzer, test, or runtime diagnostic was used.
