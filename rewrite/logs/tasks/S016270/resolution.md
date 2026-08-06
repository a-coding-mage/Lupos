# Resolution — S016270

Applier: P01 / gpt-5.6-terra (high)

## Upstream recheck

Reopened the complete pinned header
`vendor/linux/include/uapi/linux/netfilter/nf_conntrack_common.h` at revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the frozen common x86_64/AArch64
scope record, the candidate, and both independent review reports. The source
contains four tagged C enum declarations, all of their global enumerators and
aliases, three state macros, four `__KERNEL__` conditional regions, and the
expectation-flag macros. There is no storage ownership, allocation, locking,
callback, refcount, RCU, or cleanup behavior in this declarative UAPI header.

## Review finding dispositions

1. **Parity P1 / Rust HIGH — SPDX mismatch: resolved.** The candidate now
   begins with the exact upstream identifier
   `GPL-2.0 WITH Linux-syscall-note`; immutable source, revision,
   architecture, and task provenance remain immediately below it.

2. **Parity P2 / Rust HIGH — flattened `__KERNEL__` interface: resolved.**
   `IP_CT_NEW_REPLY` is now present only under
   `#[cfg(not(feature = "__KERNEL__"))]`, while `IP_CT_UNTRACKED`,
   `IPS_NAT_CLASH_{BIT,}`, `__IPS_MAX_BIT`, `__IPCT_MAX`, and
   `NF_CT_EXPECT_{DEAD,MASK}` are present only under
   `#[cfg(feature = "__KERNEL__")]`. These are the direct Rust
   counterparts of upstream lines 30--34, 100--107, 147--149, and 162--166;
   no unconditional replacement symbol remains.

3. **Parity P3 / Rust MEDIUM — distinct tagged C enum types: resolved.**
   Each source tag (`ip_conntrack_info`, `ip_conntrack_status`,
   `ip_conntrack_events`, and `ip_conntrack_expect_events`) is now a distinct
   `#[repr(transparent)]` wrapper over `core::ffi::c_int`; every corresponding
   enumerator is constructed with its owning tag. This follows the adjacent
   common UAPI enum convention and preserves 32-bit C-integer representation,
   tag distinction, and the C allowance for integer values outside the listed
   enumerators without inventing closed Rust-enum validity requirements.

4. **Rust MEDIUM — invalid-input semantics of `NF_CT_STATE_BIT`: resolved.**
   The source expression is retained exactly as
   `1 << (ctinfo.0 % IP_CT_IS_REPLY.0 + 1)`, but is exposed as `const unsafe
   fn`. Its safety contract requires the computed C `int` shift count to be
   non-negative and valid; that is the upstream macro's precondition, because
   C has undefined behavior for a negative shift count. This preserves the
   existing caller obligation rather than creating a safe public operation
   whose invalid input could introduce Rust panic behavior. The pinned callers
   use `NF_CT_STATE_BIT` only after a valid conntrack `ctinfo` is supplied
   (`net/netfilter/nft_ct.c:70`, `nft_ct_fast.c:22`, and
   `nf_tables_trace.c:113`); their separate untracked/invalid paths select
   `NF_CT_STATE_UNTRACKED_BIT` or `NF_CT_STATE_INVALID_BIT` instead.

## Final source-only checks

- All upstream enumerator values, aliases, status masks, and expectation
  macros remain mapped, including `IP_CT_ESTABLISHED_REPLY`,
  `IP_CT_RELATED_REPLY`, `IPCT_NATSEQADJ`, `IPS_NAT_MASK`,
  `IPS_NAT_DONE_MASK`, and `IPS_UNCHANGEABLE_MASK`.
- The candidate has no placeholder, fake-success path, Rust test
  configuration, driver code, or broad unsafe block.
- No source outside the leased destination and no shared manifest/index was
  edited by this applier. No compiler, formatter, build, linker, test,
  emulator, debugger, or runtime command was run.

Every reported finding is resolved against the pinned header and its local
callers.
