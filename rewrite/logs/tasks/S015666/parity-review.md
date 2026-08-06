# Parity review — S015666 (slot 1)

Reviewed `vendor/linux/include/net/tcp_states.h` at pinned revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/net/tcp_states.rs` in full.  Scope is `common` for both frozen
targets; the source contains no selected configuration branch.

## Result

Accepted. No parity findings.

## Exhaustive mapping checked

- The anonymous state-enumerator sequence is represented as individual public
  `i32` integer constants, preserving the explicit `TCP_ESTABLISHED = 1`, each
  implicit successor through `TCP_BOUND_INACTIVE = 13`, and terminal
  `TCP_MAX_STATES = 14`. The source anonymous enum creates integer constants,
  not a named/tagged type or object, so no enum layout, tag, storage, or symbol
  contract is omitted.
- `TCP_STATE_MASK` is the same signed-int value `0xF`.
- `TCP_ACTION_FIN` retains the source expression `1 << TCP_CLOSE`, evaluating
  to signed 32-bit integer value 128 on both frozen targets.
- Every member of the second anonymous enum is present with the original
  signed-`int` shift expression: `TCPF_ESTABLISHED` through
  `TCPF_BOUND_INACTIVE`. Their shifts are 1 through 13, respectively, so all
  are defined, positive `i32` values on both targets (2 through 8192) and have
  the same C integer-promotion/bitwise results.
- The C include guard is purely a repeated-inclusion preprocessor guard; the
  Rust module supplies a single definition and the candidate introduces no
  runtime, layout, linkage, storage, conditional, or synchronization change.
- SPDX, source path, exact Linux revision, architecture, and task provenance
  match the pinned source and task row. No branding delta, test, placeholder,
  unsafe code, or extra definition was introduced.

## Consumer-context check

`net/ipv4/tcp.c` and `net/mptcp/protocol.c` combine state transitions with
`TCP_ACTION_FIN`, then recover the low state with `TCP_STATE_MASK`; the
candidate values retain those exact masks and action bit. `net/ipv4/tcp_diag.c`
uses `TCPF_SYN_RECV`, `TCPF_NEW_SYN_RECV`, `TCPF_LISTEN`, and
`TCPF_BOUND_INACTIVE` as state-selection masks. The candidate preserves each
corresponding integer bit and all other `TCPF_*` state masks used by TCP, MPTCP,
and IPv6 TCP paths.
