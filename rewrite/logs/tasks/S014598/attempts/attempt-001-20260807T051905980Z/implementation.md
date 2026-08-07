# S014598 implementation

- Source: `vendor/linux/include/linux/pci_ids.h` at Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Destination: `src/include/linux/pci_ids.rs`.
- Scope: all 2,902 object-like `#define` entries in the pinned header; no function-like macros or conditional branches are present.
- Translation: each Linux macro is represented as a `pub const` with `u32` type and the original literal/identifier expression preserved. Header comments and SPDX provenance are retained as Rust comments.
- Architectures: x86_64,aarch64 (task scope is common).

No compiler, formatter, test, or runtime command was run.
