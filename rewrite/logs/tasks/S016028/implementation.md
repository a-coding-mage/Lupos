# S016028 implementation

- Task: `S016028`
- Pipeline/attempt: `P02` / `1`
- Linux source: `vendor/linux/include/uapi/asm-generic/termbits-common.h`
- Destination: `src/include/uapi/asm-generic/termbits-common.rs`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architectures: `common`

The destination is a fresh path-preserving translation of the complete pinned
header. The two C typedefs are represented with the corresponding fixed-width
Rust integer types (`u8` and `u32`). Every selected macro is represented as a
public constant, including the alias constants `EXTA` and `EXTB`, and the
original section grouping and values are retained. Constants preserve the C
literal signedness: values representable as `int` use `i32`, while
`CRTSCTS` uses `u32` because `0x80000000` is an `unsigned int` hexadecimal
constant under the frozen targets. The include guard has no Rust artifact.

No compiler, formatter, linker, test, runtime, or historical Lupos source was
used.
