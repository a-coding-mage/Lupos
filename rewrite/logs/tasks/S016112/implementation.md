# S016112 implementation

- Task: `S016112`
- Linux source: `vendor/linux/include/uapi/linux/elf-em.h`
- Destination: `src/include/uapi/linux/elf-em.rs`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architectures: `common`
- Scope class: `RUST_TRANSLATE`

The complete pinned header was read. It has no includes, functions, types, or
callers requiring additional translation context. The selected operative
symbols are the include guard and all 49 `EM_*` integer macros listed in
`rewrite/SYMBOLS.tsv`, including duplicate-valued historical aliases and the
four hexadecimal values. The include guard is represented by the Rust module
boundary; each selected numeric macro is represented as a public `i32`
constant, preserving its value and exported spelling. The source SPDX notice,
comments, source path, pinned revision, common architecture marker, and task
provenance are retained.
