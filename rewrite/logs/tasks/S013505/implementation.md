# Implementation — S013505

Source: `vendor/linux/include/linux/bcma/bcma_regs.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The source has an include guard and 74 object-like constant macros; it contains
no types, functions, storage, conditionals other than the guard, or runtime
behavior. The Rust translation exposes every constant under its original name.
Unsuffixed C integer literals and the parenthesized `64 * 1024 * 1024`
expression retain `i32`; literals carrying C's `U` suffix retain `u32`.
The reversed BCM4328 clock-bit names are separately retained despite their
equal numeric values. The include guard has no Rust runtime or module analogue.

No compiler, formatter, linker, test, or runtime command was run.
