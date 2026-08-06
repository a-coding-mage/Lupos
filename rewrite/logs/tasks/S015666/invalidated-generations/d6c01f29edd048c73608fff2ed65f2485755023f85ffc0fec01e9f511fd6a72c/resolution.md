# Resolution — S015666

Pinned source re-opened: `vendor/linux/include/net/tcp_states.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, complete lines 1–50. Candidate:
`src/include/net/tcp_states.rs`.

## Review dispositions

1. Parity review: accepted, no findings. Confirmed. The first anonymous enum
   supplies the ordered signed-`int` enumerator values 1 through 14; the Rust
   public `i32` constants retain every name and value. `TCP_STATE_MASK` remains
   `0xF`, and `TCP_ACTION_FIN` remains the signed-`int` expression
   `1 << TCP_CLOSE` (128). The second anonymous enum retains every
   `TCPF_* = 1 << TCP_*` expression with shift counts 1 through 13, hence the
   same positive `i32` results on both frozen targets. Pinned consumers confirm
   the required masks and state/action encoding: `net/ipv4/tcp.c:3037-3061`,
   `net/mptcp/protocol.c:3197-3218`, `net/core/sock_map.c:541-551`, and
   `net/ipv4/tcp_diag.c:392-396`.
2. Rust review: accepted, no findings. Confirmed. Neither anonymous enum
   defines a named tag, storage object, exported linker symbol, ownership or
   lifetime contract. Individual `pub const i32` definitions preserve the C
   enumerators' integer-constant semantics without inventing a Rust enum,
   representation, allocation, `unsafe`, or an FFI surface. All stated shifts
   are representable in `i32`.

## Manifest closure

All fourteen `SYMBOLS.tsv` rows now identify the always-selected enum/macro
semantics or the inclusion-guard-only preprocessor boundary for both frozen
targets. All four `LIFETIMES.tsv` rows are complete: both anonymous enums have
no object ownership, runtime lifetime, or locking/RCU/refcount behavior. All
four `ABI.tsv` rows are complete: no linkage, export, runtime layout, alignment,
or calling convention exists; the only interface is signed `int` constants
represented by `i32` constants in Rust. This header has no task rows in
`DRIVER_ABI.tsv` or `BLOCKERS.tsv`; those families are not applicable.

No source change was required during application: the candidate already
preserves the complete pinned header. No build, formatter, linker, test, or
runtime command was run.
