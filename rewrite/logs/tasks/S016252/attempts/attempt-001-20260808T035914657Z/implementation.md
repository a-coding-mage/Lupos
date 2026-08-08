# S016252 implementation

Pinned source: `vendor/linux/include/uapi/linux/mptcp_pm.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The source is an unconditional generated UAPI declaration header. The Rust
translation preserves both named enum namespaces as `u32` aliases, every
enumerator value including explicit gaps, each anonymous-enum constant, and
each `*_MAX` expression as its original terminal constant minus one. No
runtime behavior, storage layout, configuration conditional, allocation, or
locking path exists in this header.

The C include guard is intentionally not represented: Rust module inclusion
provides the corresponding one-definition protection and the guard is not a
UAPI value.
