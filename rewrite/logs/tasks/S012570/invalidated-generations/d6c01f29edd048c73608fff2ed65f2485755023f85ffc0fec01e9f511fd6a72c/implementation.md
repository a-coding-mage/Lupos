# S012570 implementation

Implemented `src/include/asm-generic/percpu_types.rs` from the complete pinned
`include/asm-generic/percpu_types.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The selected x86_64 and AArch64 configurations both include this generic
header.  Its sole operative C macro, `__percpu_qual`, is guarded so it supplies
an empty replacement list only when an architecture has not already supplied
one.  The direct consumer is `include/linux/compiler_types.h`, where it
precedes the compiler-only `BTF_TYPE_TAG(percpu)` annotation in `__percpu`.
The direct x86 context is `arch/x86/include/asm/percpu_types.h`, which may set
the qualifier to its segment override before including this generic file.

No Rust item is introduced: an empty C preprocessor qualifier has neither a
Rust type/layout/linkage/runtime equivalent nor a call-site API.  The
destination records that absence and preserves the architecture-specific
override boundary rather than inventing a Rust marker or macro.  No compiler,
formatter, linker, test, or historical Lupos source was used.

Implementation role/model: implementer, gpt-5.6-terra, medium effort.
