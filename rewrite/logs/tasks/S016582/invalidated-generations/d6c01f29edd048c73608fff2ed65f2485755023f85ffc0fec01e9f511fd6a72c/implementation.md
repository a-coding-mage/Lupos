# Implementation — S016582

Mapped `include/xen/interface/io/xenbus.h` at pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` to
`src/include/xen/interface/io/xenbus.rs` for AArch64.

The source contains one unconditional `enum xenbus_state`. Its nine explicitly
valued protocol states (0 through 8) are exposed with their original names.
The Rust representation is a transparent `i32` newtype, preserving the Linux
C enum's `int` ABI and its ability to contain values other than the declared
enumerators. No source conditional, macro with runtime effect, function, or
other type is present.

Evidence consulted: the frozen AArch64 configuration (with `CONFIG_XEN=y`),
the header-closure and include-edge records, task S016582 symbol/lifetime/ABI
rows, and the Xenbus consumer declaration in `include/xen/xenbus.h`.
