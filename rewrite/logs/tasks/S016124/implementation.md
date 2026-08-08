# S016124 implementation

Translated `include/uapi/linux/falloc.h` from pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` to
`src/include/uapi/linux/falloc.rs` for the common x86_64/AArch64 scope.

The source header has only its include guard and nine unconditional UAPI
integer macro definitions; it has no includes, functions, types, structures,
or configuration-dependent branches.  The Rust translation preserves each
macro's public name and `int`-typed hexadecimal value as an `i32` constant:
the allocate default (`0x00`), keep-size (`0x01`), punch-hole (`0x02`), the
reserved no-hide-stale codepoint (`0x04`), collapse (`0x08`), zero (`0x10`),
insert (`0x20`), unshare (`0x40`), and write-zeroes (`0x80`).

Pinned direct-context evidence: `vendor/linux/include/linux/falloc.h:1-42`
imports this UAPI header and combines its modes; selected users include
`vendor/linux/block/fops.c:840-893` and `vendor/linux/fs/open.c:259-294`.
The frozen scope row records header-closure selection through `block/fops.o`
for both configurations.

No compiler, formatter, linker, test, runtime, or historical-source command
was used.
