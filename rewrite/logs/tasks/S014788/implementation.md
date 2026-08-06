# S014788 implementation record

Oracle: `vendor/linux/include/linux/rational.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The header is unconditional for both frozen configurations. It contributes
one declaration: `rational_best_approximation`. The source implementation is
the separate queued task `S017291` (`lib/math/rational.c`); this header does
not contain its continued-fraction arithmetic and does not reimplement it.

The declaration is preserved as an `unsafe extern "C"` item. Each C
`unsigned long` is `core::ffi::c_ulong`, which retains the target C ABI width
for both frozen 64-bit architectures. Its two output parameters remain mutable
raw pointers so nullability, aliasing, and writable-storage obligations remain
those of the C interface rather than being strengthened into Rust references.

No selected configuration condition, allocation, ownership transfer,
synchronization, or arithmetic operation occurs in this header.
