# Implementation: S016428

Source: `vendor/linux/include/uapi/linux/tty_flags.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

Created `src/include/uapi/linux/tty_flags.rs` as the path-preserving Rust UAPI
translation.  Every `ASYNCB_*` position macro is represented as `c_int`, which
matches the C unsuffixed integer literals.  Every `1U << ...` flag and derived
mask is represented as `u32`, preserving the C unsigned-int width used by the
source on the approved architectures.

The source's `__KERNEL__`-excluded obsolete UAPI names are retained as public
constants: the Rust UAPI surface has no C preprocessor kernel/userspace split,
while the names and their original values remain available to UAPI consumers.
No types, functions, storage, or conditional configuration behavior exists in
the source beyond these macros.

No build, formatter, test, or runtime command was run.
