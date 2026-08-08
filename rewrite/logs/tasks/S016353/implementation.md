# S016353 implementation evidence

The required branch was verified as `feat/bun-like-rewrite-test` before the
destination and evidence files were written.  The pinned implementation oracle
was `vendor/linux/include/uapi/linux/reboot.h` at Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The complete header contains only the UAPI include guard, explanatory comments,
and thirteen object-like reboot macros.  The fresh common-architecture Rust
destination preserves every macro name and value as a public `u32` constant:
the five magic values and eight reboot command values.  The C header has no
functions, types, conditional configuration branches, ABI layout, or
architecture-specific body to translate.  The include guard is preprocessing
state and therefore has no Rust declaration.

The destination is `src/include/uapi/linux/reboot.rs`, with immutable source,
revision, architecture, and task provenance in its first lines.  No compiler,
formatter, linker, test, runtime, or historical Lupos source was used.
