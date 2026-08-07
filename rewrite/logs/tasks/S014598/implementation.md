# Implementation S014598

- Task: `S014598`; pipeline `P01`; attempt `1`; lease owner `codex-root-repair-20260807-p01`.
- Branch: `feat/bun-like-rewrite-test`.
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Source: `vendor/linux/include/linux/pci_ids.h` (SHA-256 `55928d1f2c4f7e6b912c54baf64b876fc8cd4d083d2016a45a87ca33ebc9439d`).
- Destination: `src/include/linux/pci_ids.rs` (SHA-256 `9e2d27850150685d36b9f50f232ae4594903467f06641a3f337acb532085ac51`).
- Architectures: `common`; source class `RUST_TRANSLATE`.
- Translation: every 2,902 numeric `#define` macro in the pinned header is represented as a `pub const` with the original hexadecimal spelling and Rust `i32` type, preserving C unsuffixed-hex `int` semantics. The empty include guard is intentionally not exported.
- No compiler, formatter, linker, test, or runtime command was run.
