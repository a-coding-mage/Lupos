# S016252 resolution — attempt 2 / P02

## Result: BLOCKED — no `DONE` or source application

The sealed attempt-2 proposal is bound to candidate SHA-256
`f5cf34e797b5f2d9d59226c2e2298968dec0aa922377e3c05e61560f05b537c2`
and proposal SHA-256
`85a2009a62269c36fb4df4c2f9a0ffe66e43376d801edfd425802ef9ac7f205e`.
Every correction below would change the candidate and invalidate those sealed
candidate/evidence bindings. Therefore no source or evidence snapshot was
altered in this attempt. A fresh implementation/review attempt is required if
the ABI blocker is resolved by permitted Phase-0 evidence; it must recreate the
candidate and all bound artifacts before any later application.

The candidate itself has three defects, but only the string and SPDX defects
are completely resolvable from the allowed source evidence. The exact storage
size and alignment of the two named C enum types for the frozen compiler and
both target configurations are not established by the permitted records: their
`rewrite/ABI.tsv` rows remain `PENDING_REVIEW`. The pinned header declares the
types but supplies no representation attribute or layout assertion. No
compiler, compiler diagnostic, or unapproved ABI assumption may fill that gap.

## Finding dispositions

### F001 — ACCEPTED; merged with RUST-001; BLOCKED on enum ABI

`include/uapi/linux/mptcp_pm.h:44-56` declares the selected named type
`enum mptcp_event_type`; the current candidate provides only its enumerator
constants and no public named representation. Direct pinned context confirms
that the type is operative: `net/mptcp/protocol.h:1161-1162` declares
`mptcp_event(enum mptcp_event_type type, ...)`, and
`net/mptcp/pm_netlink.c:572-573` defines it.

The correction must provide the named type in addition to preserving the
listed values and gaps. However neither the header nor the permitted frozen ABI
row proves its frozen x86_64/AArch64 size or alignment. The proposed source
representation is consequently not source-proven. This finding remains the
blocking issue.

### F002 — ACCEPTED IN PART; source correction required, but its direct-use rationale is narrowed

The candidate's `pub const MPTCP_PM_NAME: &str = "mptcp_pm";` omits the C
string literal's terminating NUL and changes its representation. Pinned
`include/uapi/linux/mptcp_pm.h:10` defines `MPTCP_PM_NAME` as `"mptcp_pm"`.
The direct pinned use at `net/mptcp/pm_netlink.c:630-632` initializes
`mptcp_genl_family.name`; `include/net/genetlink.h:78-81` establishes that
member as `char name[GENL_NAMSIZ]`, an inline array. Thus the review's claim
that this direct initializer requires a pointer expression is not supported by
the caller: it requires the literal's NUL-terminated character-array contents.

A fresh candidate must retain an ABI-facing NUL-terminated nine-byte static
representation (`mptcp_pm` plus `\\0`), with any Rust string view separate.
Whether a pointer-facing helper is needed must be decided only from a selected
translated caller; none was established in the permitted direct context.
This correction invalidates the sealed candidate hashes, so it cannot be
applied in attempt 2.

### F003 — ACCEPTED; source correction required

Pinned `include/uapi/linux/mptcp_pm.h:1` is
`SPDX-License-Identifier: ((GPL-2.0 WITH Linux-syscall-note) OR BSD-3-Clause)`.
The candidate instead begins with `GPL-2.0-only`. A fresh candidate must retain
the exact upstream dual SPDX identifier. This correction invalidates the
sealed candidate hashes and cannot be applied in attempt 2.

### RUST-001 — ACCEPTED; duplicate of F001; BLOCKED on enum ABI

The named `enum mptcp_event_type` omission is independently confirmed as
described for F001. A closed Rust enum would additionally need proof that it
preserves every C value accepted at the type boundary; the current source and
frozen ABI records establish neither that nor the underlying layout. No
representation may be chosen by assumption.

### RUST-002 — ACCEPTED; BLOCKED on enum ABI

Pinned `include/uapi/linux/mptcp_pm.h:110-132` declares the separately selected
named `enum mptcp_event_attr`, while the candidate only exports `MPTCP_ATTR_*`
constants. It needs a public named representation. As with F001, the allowed
source evidence and still-pending frozen ABI records do not establish its
target size/alignment or a representation that safely preserves all C values.
This is a second instance of the same ABI blocker, not a basis to invent a
restrictive Rust enum.

### RUST-003 — ACCEPTED; duplicate of F002

The `&str` representation is not the pinned C string literal and lacks its
NUL terminator. The direct caller proves inline-array initialization rather
than a direct pointer field; the required fresh-candidate correction is the
NUL-terminated byte representation stated under F002.

## Required coordinator action

Use the queue tool to mark `S016252` `BLOCKED` with the reason: `named C enum
representations are selected but their frozen x86_64/aarch64 ABI
(size/alignment/valid-value representation) is still PENDING_REVIEW and cannot
be established from permitted Phase-1 source evidence`. Do not mark `DONE` and
do not perform an attempt-2 source correction. If later authorized Phase-0 ABI
evidence closes that blocker, requeue for a fresh attempt; that attempt must
also correct the NUL-terminated `MPTCP_PM_NAME` representation and exact SPDX
identifier, then regenerate sealed implementation/candidate/proposal/review
evidence.
