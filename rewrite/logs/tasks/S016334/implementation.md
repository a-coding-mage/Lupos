# S016334 implementation

- Task: `S016334`
- Pipeline/attempt: `P02` / `1`
- Linux source: `vendor/linux/include/uapi/linux/posix_acl.h`
- Destination: `src/include/uapi/linux/posix_acl.rs`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architectures: `common` (selected for both frozen x86_64 and AArch64 configurations)

The pinned UAPI header contains no data declarations. It defines one signed
integer sentinel and eleven integer ACL constants. The translation preserves
each macro name and value as a public `i32` constant. The C include guard is
represented by the Rust module boundary and is not emitted as a symbol. No
struct, enum, layout, or lifetime representation was invented.

Source-level checks performed against the pinned header and frozen symbol
inventory: all 12 operative macros are present, values are unchanged, and the
architecture-specific symbol rows are identical.
