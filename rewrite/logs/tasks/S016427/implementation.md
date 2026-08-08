# S016427 implementation

- Task: `S016427`
- Pipeline/attempt: `P01` / `1`
- Linux source: `vendor/linux/include/uapi/linux/tty.h`
- Destination: `src/include/uapi/linux/tty.rs`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architectures: `common` (selected by both frozen configurations)
- Scope evidence: `rewrite/SCOPE.tsv` and `rewrite/metadata/header_closure.tsv`
- Symbol evidence: `rewrite/SYMBOLS.tsv` rows for S016427

The complete pinned header contains only its SPDX notice, a header guard, a
comment, and integer-valued line-discipline macros. The Rust destination keeps
the SPDX notice and immutable provenance, represents each C integer constant as
an `i32` constant (the C literal type is `int`), and preserves every name and
numeric value from `N_TTY` through `N_CAN327` and `NR_LDISCS`. The C header guard
has no emitted Rust item because Rust module inclusion supplies the equivalent
single-definition boundary; it introduces no runtime behavior or ABI object.

No configuration-specific branch, type, function, allocation, synchronization,
or unsafe operation is present in the pinned source. No branding change was
made. No compiler, formatter, build, test, or runtime command was run.
