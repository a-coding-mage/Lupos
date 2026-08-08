# Implementation evidence

- task: S015666
- attempt: 1
- pipeline: P01
- source: `vendor/linux/include/net/tcp_states.h`
- destination: `src/include/net/tcp_states.rs`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- architectures: `common` (x86_64 and AArch64)

The pinned header defines two anonymous C enums, `TCP_*` state values and
`TCPF_*` bit masks, plus `TCP_STATE_MASK` and `TCP_ACTION_FIN`. The translation
keeps each exported name and value, uses `i32` because C enumerators and the
integer shift expressions have C `int` semantics in this header, and retains
the shift expressions for derived values rather than replacing computed state
with literals. The anonymous C enum types are not represented as named Rust
enums because the frozen ABI records do not establish a named enum type.

No compiler, formatter, test, runtime, or historical Lupos source was used.
