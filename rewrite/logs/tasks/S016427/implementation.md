# Implementation: S016427

Translated `include/uapi/linux/tty.h` from pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` to
`src/include/uapi/linux/tty.rs`.

The source has no includes, architecture/configuration branches after header
guards, functions, structures, or ABI-bearing layouts. It defines 31
line-discipline number macros and `NR_LDISCS`. Every macro is represented as a
public `core::ffi::c_int` constant with the same identifier and value; the C
unsuffixed integer literals have C `int` type on both frozen architectures.

No source outside the leased destination and this task's evidence directory was
edited. No build, compiler, formatter, test, linker, or runtime command ran.
