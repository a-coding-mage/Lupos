# Implementation — S015666

Source: `vendor/linux/include/net/tcp_states.h` at `425f94c2954b1fe80ebdbf9b29854e89750355df`.

The complete selected header is mapped to `src/include/net/tcp_states.rs`.
Both anonymous C enums are represented by individually named `i32` constants:
the frozen x86_64 and AArch64 targets use 32-bit C `int`, and the header's
consumers compare and shift the enumerator values as integer constants.
The ordered TCP state sequence is preserved exactly, including
`TCP_BOUND_INACTIVE` and the terminal `TCP_MAX_STATES`; `TCP_STATE_MASK`,
`TCP_ACTION_FIN`, and all `TCPF_*` shift expressions are retained with their
original integer operands and values. The header has no configuration branches,
storage, ownership, locking, FFI linkage, or lifetime behavior.

Context checked: `include/net/sock.h` includes this header; `net/ipv4/tcp.c`
asserts the internal state values match the BPF UAPI values and uses the values
as `int` state arguments. No branding allowlist entry or task-specific blocker
applies.
