# Implementation — S016378 attempt 2

- Source: `vendor/linux/include/uapi/linux/serial_reg.h` at Linux revision
  `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Frozen scope: `RUST_TRANSLATE`, common (`x86_64,aarch64`), destination
  `src/include/uapi/linux/serial_reg.rs`; no dependencies.
- Translation: every object-like macro is represented as a public constant.
  Values that are C `int` expressions use `i32`; the three OMAP base literals
  are C `unsigned int` and use `u32`. Derived definitions retain their source
  arithmetic, bitwise operators, and precedence.
- The one function-like macro evaluates its argument once upstream and is
  represented by an equivalently named Rust macro. Its unsuffixed literals
  retain contextual integer typing, matching the C usual arithmetic
  conversions for the argument expression.
- The C include guard has no Rust analogue and was intentionally omitted.
- This task has no types, storage, ownership, locking, ABI layout, or
  configuration branches beyond the header guard.
