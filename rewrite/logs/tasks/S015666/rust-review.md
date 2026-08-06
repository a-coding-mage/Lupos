# Rust review — S015666

Reviewed `src/include/net/tcp_states.rs` against pinned
`vendor/linux/include/net/tcp_states.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, plus the frozen task records and
representative pinned consumers (`include/net/sock.h`, `net/mptcp/protocol.c`,
and `net/core/sock_map.c`).

## Result

No Rust-specific findings.

## Audit record

- The source's two anonymous enums introduce enumerator identifiers rather
  than a named, layout-bearing C enum type.  The candidate exposes exactly the
  fourteen state values and thirteen state-flag values as public `i32`
  constants.  This preserves the C enumerator integer-constant representation
  on both frozen targets without inventing a Rust enum, `repr`, storage, or
  FFI surface.
- `TCP_STATE_MASK` is retained as `0xF`; `TCP_ACTION_FIN` and each `TCPF_*`
  expression retain a signed 32-bit left operand and the original shift count.
  The greatest shift is `1 << TCP_BOUND_INACTIVE` (`1 << 13`), so every value
  is representable in `i32` and cannot overflow or panic in Rust constant
  evaluation.  The result set is the same as the C `int` expressions.
- Pinned consumers use these `int` enumerators with `u8` socket states and,
  in places such as diagnostic state masks, `u32` values.  C applies its usual
  integer conversions at those use sites; the fixed `i32` public constants are
  correct for the source enum declarations.  Each future Rust consumer must
  make the corresponding conversion at its own operation boundary rather than
  changing these declarations' type.
- The C include guard has no runtime or ABI content; replacing it with the
  Rust module's one-definition boundary omits no selected conditional branch.
  This header has no configuration predicates, data objects, ownership,
  pointer, synchronization, allocation, `unsafe`, layout, or linkage behavior.
- Provenance names the correct Linux path, revision, common architecture
  scope, and task.  The candidate contains no tests, panic/placeholder,
  unchecked indexing, allocation, `unsafe`, or mutable state.

The applier still needs to close the task's `PENDING_REVIEW` manifest fields
with these conclusions before `DONE`.
