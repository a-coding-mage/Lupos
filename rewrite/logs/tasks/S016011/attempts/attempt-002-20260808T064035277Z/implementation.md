Task S016011 implementation evidence (attempt 2, P01)

Source: vendor/linux/include/uapi/asm-generic/mman-common.h
Destination: src/include/uapi/asm-generic/mman-common.rs
Linux revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
Architectures: common (x86_64 and aarch64 consumers)

The complete pinned header was read. It contains only an include guard,
comments, and numeric UAPI macros. Rust module loading supplies the include
guard's module-once behavior; no guard constant is fabricated. Every selected
numeric macro is represented as a public signed `i32` constant, preserving
the unsuffixed C expression type. `PKEY_ACCESS_MASK` remains a computed OR of
the two preceding constants, preserving Linux's expression and dependency.

No callers, callees, locking, allocation, ABI layout, or Kbuild behavior is
introduced by this constants-only header. No compiler, formatter, test, or
runtime command was run.
