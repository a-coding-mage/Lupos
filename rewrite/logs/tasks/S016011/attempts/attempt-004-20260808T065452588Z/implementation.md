# S016011 implementation evidence

- Pipeline: P01; attempt: 4; role: implementer; model: Terra; reasoning effort: medium.
- Branch verified as `feat/bun-like-rewrite-test`.
- Pinned source: `vendor/linux/include/uapi/asm-generic/mman-common.h`.
- Pinned revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Source SHA-256: `52849c062488fdaaa877e7a491bec38643e732dc04baa2f6e15b365ab77eb504`.
- Destination: `src/include/uapi/asm-generic/mman-common.rs`.
- Destination SHA-256: `33b3b400770c65e3fe8ed2f02d7b7f4bdd62c44c4794bcaff36645b62adc7fe5`.
- Architecture membership: `common`.
- The complete 94-line header was read. Its 54 numeric `#define` symbols are represented as `i32` constants, including all compatibility and madvise values. `PKEY_ACCESS_MASK` remains computed from `PKEY_DISABLE_ACCESS | PKEY_DISABLE_WRITE`. The C include guard is mapped to one Rust module declaration.
- No compiler, formatter, linker, test, runtime, or historical Lupos source was used.

