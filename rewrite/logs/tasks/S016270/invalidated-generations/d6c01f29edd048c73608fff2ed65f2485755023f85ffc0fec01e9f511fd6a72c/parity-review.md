# S016270 parity review

Reviewer: parity reviewer (independent)

## Scope and evidence

Reviewed the complete pinned source
`vendor/linux/include/uapi/linux/netfilter/nf_conntrack_common.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` against only
`src/include/uapi/linux/netfilter/nf_conntrack_common.rs`.

The queue row identifies the task as common x86_64/AArch64 UAPI.  The source
contains four tagged C enums, their enumerators and aliases, three state
macros, expectation macros, and four `__KERNEL__` conditional regions.

## Findings

1. **Must fix — SPDX identifier changed.**  The upstream header begins with
   `GPL-2.0 WITH Linux-syscall-note` (source line 1), while candidate line 1
   states `GPL-2.0-only`.  This is neither the exact upstream SPDX identifier
   nor an allowlisted branding change.  Restore the upstream identifier.

2. **Must fix — `__KERNEL__` branches were flattened and the userspace
   enumerator is omitted.**  Source lines 30–34 select
   `IP_CT_NEW_REPLY = IP_CT_NUMBER` for non-kernel consumers and
   `IP_CT_UNTRACKED = 7` only for `__KERNEL__`; the candidate unconditionally
   exposes the latter (line 24) and provides no `IP_CT_NEW_REPLY`.  Likewise,
   source lines 100–107, 147–149, and 162–166 expose
   `IPS_NAT_CLASH_{BIT,}`, `__IPCT_MAX`, and
   `NF_CT_EXPECT_{DEAD,MASK}` only under `__KERNEL__`, whereas candidate lines
   68–69, 105, and 116–118 make each unconditional.  Preserve all four source
   conditional regions with the project’s Rust equivalent (as used by the
   neighboring UAPI translation’s `#[cfg(feature = "__KERNEL__")]`) and
   retain the non-kernel `IP_CT_NEW_REPLY` alternative.  The frozen kernel
   contexts selecting the kernel arms does not authorize erasing the header’s
   stated compile-time interface.

3. **Must fix — four distinct tagged C enum types collapse into `i32`
   aliases.**  The source declares distinct `enum ip_conntrack_info`,
   `enum ip_conntrack_status`, `enum ip_conntrack_events`, and
   `enum ip_conntrack_expect_events` (source lines 7, 42, 133, and 152).
   Candidate lines 15, 35, 90, and 107 make all four direct `i32` aliases,
   eliminating the source-level type distinction for FFI/API consumers.  Use
   one transparent C-`int` newtype per tag (the established UAPI pattern in
   `src/include/uapi/linux/netdev.rs`) and construct each enumerator with its
   owning type, while retaining all numeric values and aliases.

## Verified portions

- The candidate retains the numeric values for the unconditional state bits,
  status bits/masks, event enumerators through `IPCT_SYNPROXY`, expectation
  event enumerators, and expectation flags.
- `IP_CT_ESTABLISHED_REPLY`, `IP_CT_RELATED_REPLY`, `IPCT_NATSEQADJ`,
  `IPS_NAT_MASK`, `IPS_NAT_DONE_MASK`, and `IPS_UNCHANGEABLE_MASK` retain
  their upstream expressions/values.
- Provenance path, pinned revision, common architecture tag, and task ID are
  correct.  No test, driver implementation, placeholder, or branding change
  other than the SPDX discrepancy was found.

No compiler, formatter, linker, test, emulator, debugger, or runtime command
was run.
