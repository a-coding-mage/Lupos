# Implementation S016189

- Source: `vendor/linux/include/uapi/linux/input-event-codes.h`
- Destination: `src/include/uapi/linux/input-event-codes.rs`
- Revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architectures: `common`
- Pipeline/lease: `P01`

Translated the complete pinned UAPI header (all 795 active `#define` symbols,
including aliases and computed count expressions) into ordered public Rust
`i32` constants. Header guard directives were omitted as Rust has module-level
uniqueness; comments, names, values, aliases, and ordering were retained from
the source. No conditional branches are active beyond the source guard.

No tests, compiler, formatter, or runtime commands were run.
