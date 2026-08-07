# Implementation — S016384, attempt 4

- Source: `vendor/linux/include/uapi/linux/snmp.h`
- Destination: `src/include/uapi/linux/snmp.rs`
- Pinned Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architectures: `x86_64,aarch64`

The complete pinned header was read directly. Its eight anonymous enum declarations are represented by 296 public `i32` constants in source order, including all eight trailing `__*MAX` terminators. Each enum's implicit numbering starts at its explicit zero member and increments exactly as in C. The two selected `512` macros are represented by public `i32` constants. The result therefore has 298 Rust constants and no runtime behavior, allocation, ownership, locking, ABI object layout, or unsafe operation.

No compiler, formatter, linker, test, diagnostics tool, historical Rust source, prior S016384 attempt evidence, or archive was used.
