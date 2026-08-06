# Implementation — S016394

- Lease verified for `P02`; the destination is `src/include/uapi/linux/sunrpc/debug.rs`.
- Oracle read: `vendor/linux/include/uapi/linux/sunrpc/debug.h` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Translated all thirteen RPC debug-mask macros as public `i32` constants. The
  unsuffixed hexadecimal C literals are representable as C `int` on both
  frozen architectures.
- Translated the anonymous C enum as eight public `i32` constants, preserving
  its explicit initial value and implicit sequential increments (1 through 8).
- The include guard has no Rust runtime or item-level equivalent. No selected
  configuration conditional, allocation, ownership, FFI layout, or unsafe
  boundary exists in this header.
- No blockers identified. No build, formatter, compiler, test, or runtime tool
  was run.
