# Applier resolution — S015666 (attempt 1, P01)

## Result

`BLOCKED`. The sealed candidate is not edited. The pinned source establishes
the numeric spellings, but the frozen Phase-0 records and selected consumer
contexts do not establish the exact Rust representation required to preserve
the C header's integer-expression, anonymous-enum, and preprocessor-guard
contracts across the frozen x86_64/AArch64 subset.

## Evidence reopened

- Pinned source: `vendor/linux/include/net/tcp_states.h:9-50` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Direct consumers: `vendor/linux/net/ipv4/tcp.c:3037-3061`,
  `vendor/linux/net/mptcp/protocol.c:3197-3221`,
  `vendor/linux/net/ipv4/tcp_diag.c:317-319`, and
  `vendor/linux/net/smc/af_smc.c:1621-1628`.
- Frozen records: `rewrite/SYMBOLS.tsv`, `rewrite/ABI.tsv`, and
  `rewrite/LIFETIMES.tsv` rows for `S015666`.
- Candidate and bound snapshot: `src/include/net/tcp_states.rs` and
  `rewrite/logs/tasks/S015666/candidate.diff`.

## Finding dispositions

### RUST-1 — fixed `i32` constants

**Accepted; unresolved.** The candidate makes every enumerator and macro an
`i32` constant. The original header instead supplies C enumeration constants
and integer constant expressions that take part in their consumer's C
conversion context. The reopened consumers demonstrate materially distinct
contexts: `tcp.c` and `protocol.c` use values in `unsigned char` designated
array initializers and `int` masks; `tcp_diag.c` combines `TCPF_*` with a
`u32`; and `af_smc.c` uses a dynamically shifted literal with `TCPF_*` masks.
The frozen `SYMBOLS.tsv` records retain the macro selection expressions and
the TCPF mechanical values as `PENDING_REVIEW`. No frozen source record maps
those C expression/conversion contracts to the candidate's monomorphic Rust
`i32` interface. Selecting casts or a replacement macro mechanism would be a
new, unreviewed design rather than an established translation.

### RUST-2 — anonymous-enum ABI and lifetime closure

**Accepted; unresolved.** Both `enum { ... }` declarations are anonymous and
create the enumerator identifiers, but the frozen ABI rows for
`anonymous_enum@12` and `anonymous_enum@34` retain `layout`, `alignment`, and
`export_kind` as `PENDING_REVIEW` on *both* x86_64 and AArch64. The matching
frozen lifetime rows retain `ownership`, `lifetime_contract`, and
`locking_rcu_refcount` as `PENDING_REVIEW`. The candidate's standalone `i32`
constants do not close those records, and the pinned header/consumer source
does not specify a Rust-visible ABI boundary or equivalent representation to
substitute. The numeric ranges alone are insufficient evidence to complete
these semantic records.

### RUST-3 — `_LINUX_TCP_STATES_H` guard

**Accepted; unresolved.** `tcp_states.h:9-10,50` defines the C multiple
inclusion guard, and frozen `SYMBOLS.tsv` records the guard condition and
operative macro for both architectures as selected `PENDING_REVIEW` items.
The candidate omits any documented/module-level mapping. Rust module loading
is not a source-proven replacement for C textual inclusion, macro visibility,
and repeated-inclusion behavior in the selected C header graph. No pinned
record establishes that equivalence, so closing it would be speculative.

## Parity-review disposition

The parity review correctly observes that all listed numeric values are
present. Its conclusion that this eliminates the anonymous-enum ABI and guard
questions is not supported by the frozen rows above; those rows remain
explicitly pending and the candidate does not furnish their mapping. The Rust
review's blocker findings therefore govern this outcome.

No compilation, formatter, analyzer, test, runtime command, or historical
Lupos source was used.
