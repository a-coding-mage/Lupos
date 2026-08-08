# S014514 implementation

Translated `include/linux/nfs_iostat.h` from pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` into
`src/include/linux/nfs_iostat.rs`.

The header has no includes, functions, storage, or conditional configuration
branches beyond its include guard. Its version string and both C enum namespaces
are represented with their original public names, C `int` value representation,
and explicit consecutive values. The count sentinels retain their source values.

No compiler, formatter, linker, test, runtime, or historical Rust source was
used.
