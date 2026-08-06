# Applier resolution — S017908

I independently reopened the complete pinned source
`vendor/linux/net/ipv6/ip6_offload.h`, its four definition sites
(`exthdrs_offload.c`, `udp_offload.c`, and `tcpv6_offload.c`), the direct
initialization and unwind consumers in `ip6_offload.c` and `af_inet6.c`, the
frozen x86_64/AArch64 configuration selections, the candidate, and both
independent review reports.  The pinned revision is
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

## Finding dispositions

1. **Parity review: accepted.** The entire source payload is exactly the four
   unconditioned external C declarations
   `int ipv6_exthdrs_offload_init(void)`, `int udpv6_offload_init(void)`,
   `int udpv6_offload_exit(void)`, and `int tcpv6_offload_init(void)`.
   The candidate declares each exactly once, under the same unmangled C symbol
   name and C calling convention, with no argument and `core::ffi::c_int`
   result.  There are no header-level attributes, storage definitions,
   configuration branches, types, inline bodies, or exports to map.
2. **Rust review: accepted.** This declaration-only module creates no Rust
   reference, layout, ownership, aliasing, pinning, allocation, cleanup,
   panic, or unwinding behavior.  The `unsafe extern "C"` boundary accurately
   expresses that these symbols enter lifecycle-sensitive kernel-global
   initialization/teardown operations; it neither changes their ABI nor
   supplies a false Rust-side lifetime guarantee.  Source consumers preserve
   the substantive ordering: `ip6_offload.c` checks TCPv6 then extension-header
   registration, while `af_inet6.c` pairs successful UDPv6 offload setup with
   `udpv6_offload_exit()` on its failure unwind.

No source correction is required.

## Final semantic closure

The six task-local `PENDING_REVIEW` `SYMBOLS.tsv` rows are all
preprocessor-only include-guard records, for each frozen architecture:

- `ifndef@7` and `endif@15` delimit the C include-once region only and have no
  generated ABI, storage, linkage, runtime, ownership, or unsafe operation.
- `__ip6_offload_h` is the corresponding one-definition preprocessor macro;
  it has no payload beyond that guard and therefore maps to Rust's single
  path-preserving module identity rather than a Rust item.

Both frozen configurations select `CONFIG_NET=y`, `CONFIG_INET=y`, and
`CONFIG_IPV6=y`; the header is mechanically selected for both x86_64 and
AArch64.  No task row exists in `ABI.tsv`, `LIFETIMES.tsv`,
`DRIVER_ABI.tsv`, or `BLOCKERS.tsv`, which is correct: this header declares no
data layout and owns no resource, lock, RCU/refcount, or driver contract.  The
task-local pending semantic facts are closed by the source evidence above.

All five required task evidence files exist.  No compiler, formatter,
rust-analyzer diagnostic, linker, test, emulator, debugger, benchmark, or
runtime command was used.
