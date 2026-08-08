# Implementation evidence

- Task: `S015671`
- Linux source: `vendor/linux/include/net/tls_prot.h`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Destination: `src/include/net/tls_prot.rs`
- Architectures: `common` (the same header is selected by both frozen configurations)

The pinned header contains three anonymous C enums and no functions, structs, or
conditional branches beyond its include guard. Every enumerator is represented
as a public `i32` constant because C enumerators have type `int`; values and
names are copied exactly. The include guard has no runtime Rust representation.

Source context confirms these constants are consumed as TLS record-type and
alert byte values by the pinned handshake, TLS, SUNRPC, and trace-event code.
No pointer, ownership, lifetime, locking, allocation, or unsafe operation is
introduced by this constants-only header translation.
