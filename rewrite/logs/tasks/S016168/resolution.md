# Application resolution — S016168, attempt 1, P01

Applied against the complete pinned
`vendor/linux/include/uapi/linux/if_infiniband.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, its selected direct context in
`vendor/linux/net/ipv6/addrconf.c:53,2344-2350`, and
`vendor/linux/include/linux/netdevice.h:2308-2314`. No compiler, formatter,
linker, test, runtime, rust-analyzer diagnostic, or historical Lupos source
was used.

The task remains BLOCKED. The Rust and parity reviews disagree on whether a
Rust module boundary plus a typed Rust constant is an exact representation of
the selected public C-preprocessor mechanisms. The pinned source establishes
the mechanisms, but supplies no bridge from the frozen single `.rs`
destination to the public C preprocessor namespace. Under the Phase 1 rule for
a semantic reviewer conflict, that cannot be accepted without an escalation;
the alternate permitted outcome is BLOCKED. The candidate is not changed.

## PR1 — public C include guard

**Disposition: accepted as a blocker.**

The upstream header uses the selected public sequence `#ifndef
_LINUX_IF_INFINIBAND_H`, `#define _LINUX_IF_INFINIBAND_H`, and `#endif` at
lines 25-30 for both frozen architectures. The current candidate instead says
that a Rust module boundary supplies the guard and emits no C preprocessor
definition. A Rust module can provide Rust inclusion structure, but no pinned
source or frozen ABI record establishes that it defines or exports the C macro
visible to UAPI consumers. Changing this scoped `.rs` file to a Rust item,
symbol, or comment would likewise not implement the C include guard. The
parity finding therefore cannot be disproved with the available local source
evidence.

Affected pending semantic records remain open: the two `ifndef` and two
`endif` records and the two `_LINUX_IF_INFINIBAND_H` operative-macro records
listed in `rewrite/SYMBOLS.tsv:369386-369393`.

## PR2 — `INFINIBAND_ALEN` macro representation

**Disposition: accepted as a blocker.**

Upstream line 28 defines the public replacement list exactly as
`#define INFINIBAND_ALEN 20`. In the selected IPv6 consumer,
`dev->addr_len != INFINIBAND_ALEN` at `addrconf.c:2346`, `addr_len` is an
`unsigned char` at `netdevice.h:2311`; the reviewer correctly identifies the
specific integer-promotion context. That evidence supports the numeric value
and the candidate's `i32` choice for this one Rust expression domain, as noted
by the Rust review, but it does not establish that a typed Rust item preserves
the public C macro replacement token or supplies it to C UAPI consumers.

The only exact source representation of that C-preprocessor behavior is the
pinned header itself; the frozen mapping permits only
`src/include/uapi/linux/if_infiniband.rs` for S016168 and no bridging C header
or generated-interface task. A source edit in the leased destination would
therefore introduce a new, unreviewed representation rather than resolve the
mechanism difference. The four `INFINIBAND_ALEN` selection/status records in
`rewrite/SYMBOLS.tsv:369389,369393` remain PENDING_REVIEW.

## PR3 — stale sealed candidate snapshot

**Disposition: accepted; not independently blocking after PR1/PR2, but it
precludes DONE.**

`candidate.diff` hashes to
`fabcad884c4e98b3bd49cfc9ec1ef4f241dc95d17fb0a01082300ae6894d5f38` and is
the snapshot bound into `semantic-closure-proposal.tsv`. It contains the
synthetic one-line notice but not the full upstream notice or the current
guard/constant rationale. The current candidate hashes to
`65ae1b379fc01d667042856ad79e82e84655ae4adf7b460e1c5cdf7f319fef62`.
Accordingly, neither the proposal nor the two review attestations is a sealed
review of the current source.

Regenerating a candidate snapshot and proposal would require a controlled
requeue and fresh independent reviews. That remedy is not used here because it
cannot resolve the preceding unresolved C-preprocessor semantic conflict; it
would only make the rejected candidate auditable. The existing sealed evidence
is retained unchanged.

## RUST-S016168-001 — stale candidate snapshot

**Disposition: accepted; same evidence-integrity result as PR3.**

The Rust review's independently recorded hashes reproduce the mismatch between
the sealed diff/proposal and the current source. Its requested requeue/fresh
review remedy is correct for evidence integrity, but a requeue would not cure
the unresolved PR1/PR2 parity conflict. No `DONE` transition is permissible.

## Final application result

No source patch is applied. `resolution.md` supplies a disposition for every
parity and Rust-review finding; the candidate, candidate snapshot, proposal,
and review reports remain unchanged as evidence. The queue must be transitioned
from APPLYING to BLOCKED with the public C include-guard / macro bridge and
the unresolved reviewer conflict as the reason. No build or test claim is made.
