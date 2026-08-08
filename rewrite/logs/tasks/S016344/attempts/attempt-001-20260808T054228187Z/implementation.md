# S016344 implementation record

- Leased task: `S016344` / `P02` attempt 1.
- Source: `vendor/linux/include/uapi/linux/psp.h` at pinned Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Destination: `src/include/uapi/linux/psp.rs`.
- Architectures: `common` (common queue row, selected by both frozen architectures).
- Frozen queue fingerprint: `cfa8dcf0d81c408de38d4202fe1ce9d9e5e301d1e67439c9530da94b1e64ee3f`.
- Phase 0 identity binding: `03f3c4afb3c7edc167ddeadac5493cbee736042cb7781182d4fdf43b2b79166d`.

The source has no executable control flow, storage, ownership, locking, or conditional configuration branch beyond its C include guard. The translation preserves the four macro values, the C-compatible `psp_version` discriminant sequence, each six anonymous-enum C-`int` enumerator sequence, every public sentinel, and each `*_MAX` subtraction relation. `pub use psp_version::*` retains the C header's global enumerator names while keeping the tagged enum type.

No compiler, formatter, linker, test, runtime command, or historical Lupos source was used.
