# S016270 applier resolution — attempt 1

Applier: P02 / gpt-5.6-terra / high reasoning effort.

I independently reopened the pinned source
`vendor/linux/include/uapi/linux/netfilter/nf_conntrack_common.h` at revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`, its direct provider wrapper
`vendor/linux/include/linux/netfilter/nf_conntrack_common.h`, and the sealed
391-record proposal bound to the current candidate.  The candidate
`src/include/uapi/linux/netfilter/nf_conntrack_common.rs` exactly preserves the
selected kernel-header semantics; no source change is required.

## Parity review disposition (slot 1)

**DISPROVED / no candidate change.** Slot 1 reports no finding.  The source
sequence is preserved as signed `i32` constants: `IP_CT_ESTABLISHED = 0`,
`IP_CT_RELATED = 1`, `IP_CT_NEW = 2`, `IP_CT_IS_REPLY = 3`, reply aliases 3
and 4, and critically `IP_CT_NUMBER = 5`.  The frozen kernel arm correctly
contains `IP_CT_UNTRACKED = 7` rather than the mutually exclusive userspace
`IP_CT_NEW_REPLY` arm.  All status bit indices, flags, aliases, NAT masks,
`IPS_UNCHANGEABLE_MASK`, event values/alias, expectation flags, and the
kernel-only status/event/expectation declarations retain their pinned values.

`NF_CT_STATE_BIT!` expands its argument once and preserves the C expression
category and grouping: `1i32 << (($ctinfo % IP_CT_IS_REPLY) + 1)`.  This
retains the signed `int` left operand and the source remainder/addition/shift
order.  The direct provider wrapper includes this UAPI header; the C include
guard is faithfully represented by the Rust module boundary.

## Rust review disposition (slot 2)

**DISPROVED / no candidate change.** Slot 2 reports no finding.  Named `i32`
aliases preserve C enum integer categories without Rust enum-validity
restrictions or a different storage declaration.  Every active numerical
expression is representable as a signed 32-bit value.  For the defined
conntrack-info inputs, `NF_CT_STATE_BIT!` shifts by 1 through 3, matching the
pinned operation and without introducing a valid replacement behavior for
the C source's invalid-input cases.  This header has no aggregates, pointers,
ownership, lifetime, synchronization, callback, FFI, allocation, or unsafe
contract; the candidate contains no `unsafe` boundary to discharge.

Both dispositions preserve the sealed same-key/same-order 391-record
proposal.  Every task-owned effective semantic record is finalized from its
reviewed proposed value; no `PENDING_REVIEW` value remains for this task.

No compiler, formatter, linker, test, runtime, emulator, debugger, or
compiler-backed diagnostic was run or used.
