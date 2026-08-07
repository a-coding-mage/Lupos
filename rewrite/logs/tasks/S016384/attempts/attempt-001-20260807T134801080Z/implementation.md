# S016384 implementation

Translated `include/uapi/linux/snmp.h` from pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` into the path-preserving
`src/include/uapi/linux/snmp.rs` for the frozen x86_64 and AArch64 union.

The source contains six anonymous C enums and two integer macros. Each enum
enumerator is represented as a public `i32` constant with its exact source-order
value; the two macros are public `i32` constants with value 512. The C include
guard has no Rust equivalent and is omitted. No conditional branch, function,
layout, ownership, allocation, synchronization, or ABI-bearing declaration is
present in this header.

Source-only comparison enumerated 298 C names and values and 298 Rust names and
values with no differences. No build, compiler, formatter, test, or diagnostic
was run.
