# Parity review — S015666 (attempt 1, P01)

## Verdict

APPROVE — no parity findings.

## Scope and evidence

- Pinned source: `vendor/linux/include/net/tcp_states.h` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Candidate: `src/include/net/tcp_states.rs` and its bound `candidate.diff`.
- Frozen task records: `rewrite/SCOPE.tsv`, `rewrite/SYMBOLS.tsv`,
  `rewrite/ABI.tsv`, `rewrite/LIFETIMES.tsv`, and the attempt-1 semantic
  proposal.
- Direct pinned consumer context: `net/ipv4/tcp.c:3037-3061`,
  `net/mptcp/protocol.c:3197-3221`, `net/ipv4/tcp_diag.c:392-396`, and
  `net/smc/af_smc.c:1621-1628`.

## Exhaustive comparison

- The first anonymous enumeration is represented as named, signed 32-bit
  constants.  Its values are exactly `TCP_ESTABLISHED = 1` through
  `TCP_BOUND_INACTIVE = 13`, followed by `TCP_MAX_STATES = 14`, matching
  `tcp_states.h:12-28`.  This header declares no named enum type or
  enum-typed object, so the candidate does not omit an object layout or a
  named enum ABI.
- `TCP_STATE_MASK` remains the integer value `0xF`.  `TCP_ACTION_FIN` and
  every `TCPF_*` value retain their source expression shape, a signed
  integer `1 <<` the corresponding state.  The greatest selected shift is
  13, so each expression is defined in the source's signed-int domain and
  has the same value in the candidate.  Direct consumers use these as the
  integer state/action mask described by `tcp.c:3037-3061` and
  `protocol.c:3197-3221`.
- The candidate exports all thirteen `TCPF_*` constants, including
  `TCPF_BOUND_INACTIVE`; the latter is consumed by `tcp_diag.c:392-396`.
  No selected macro, enum constant, conditional branch, or state-mask
  member is missing.
- The C include guard is a preprocessing multiple-inclusion control and has
  no runtime/data/ABI element to reproduce inside the path-preserving Rust
  module.  No source-level guarding or architecture conditional remains
  after the common header is selected.
- Provenance names the exact Linux path, revision, common architectures, and
  task ID.  There is no branding delta, placeholder, test configuration,
  altered algorithm, lifetime/locking behaviour, or FFI/layout-bearing
  declaration in this constants-only header.

## Semantic closure

No finding maps to an SC1 key.  The proposal's records for the two anonymous
enumerations are correctly closed as constants-only declarations: they create
no instance, ownership, locking, linkage, or layout contract beyond the
integer values reviewed above.
