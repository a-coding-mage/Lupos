# S016011 implementation

- Linux source: `vendor/linux/include/uapi/asm-generic/mman-common.h`
- Destination: `src/include/uapi/asm-generic/mman-common.rs`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architectures: `x86_64,aarch64`
- Attempt: `3`
- Pipeline: `P01`

Translated every selected macro from the complete pinned header as an `i32`
Rust constant. The `PKEY_ACCESS_MASK` expression remains computed from the two
component constants. The C include guard is represented by the Rust module
being declared once at its path. No conditional branches occur in this header.

Source evidence: `vendor/linux/include/uapi/asm-generic/mman-common.h`.
