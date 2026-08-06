# S016029 implementation

- Task: `S016029` (`include/uapi/asm-generic/termbits.h` -> `src/include/uapi/asm-generic/termbits.rs`)
- Pinned Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architectures: `common` (selected by the frozen x86_64 and AArch64 configurations)
- Dependency used: `S016028`, which supplies the source-equivalent `cc_t` (`u8`) and `speed_t` (`u32`) aliases.

The fresh Rust file preserves the UAPI type and field ordering with `#[repr(C)]`:

- `tcflag_t` is `u32`, matching C `unsigned int`.
- `termios`, `termios2`, and `ktermios` preserve their source member order; their control-character members remain `cc_t[19]` and the speed members remain `speed_t`.
- The C unsuffixed integer macro values are represented as `i32`; `NCCS` remains an `i32` and is explicitly converted only at Rust's array-length boundary.
- All 96 value macros from the source header are represented, in source grouping and value order. The C include guard is intentionally represented by Rust module inclusion rather than as a public value.

No configuration conditionals occur in the source header beyond its C include guard. No tests, build, formatter, compiler, linker, or runtime command was run.
