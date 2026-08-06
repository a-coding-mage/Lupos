# S016028 implementation

Translated `include/uapi/asm-generic/termbits-common.h` to the path-preserving
`src/include/uapi/asm-generic/termbits-common.rs` from the pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The source is unconditional for both frozen architectures. It has two UAPI
typedefs and 45 integer macros, with no functions, structs, or runtime state.
`cc_t` is preserved as `u8`; `speed_t` is preserved as `u32`. Every flag,
baud selector, alias, shift count, flow action, and flush selector is exported
under its original name. The flag constants use `u32`, the exact underlying
type of the consuming `tcflag_t` in `asm-generic/termbits.h`; `EXTA` and
`EXTB` retain the source aliases to `B19200` and `B38400`.

The C include guard has no Rust equivalent because Rust module inclusion is
resolved by the module system. The source SPDX expression and immutable
provenance are retained.

No build, formatting, test, linker, or runtime command was run.
