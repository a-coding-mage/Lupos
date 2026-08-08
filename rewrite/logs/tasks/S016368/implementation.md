# S016368 implementation — P01 / attempt 3

task: S016368  
pipeline: P01  
source: `vendor/linux/include/uapi/linux/securebits.h`  
destination: `src/include/uapi/linux/securebits.rs`  
architectures: common (`x86_64`, `aarch64`)  
linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`

The complete pinned UAPI header was read, together with the selected wrapper
`vendor/linux/include/linux/securebits.h` and direct uses in
`vendor/linux/security/commoncap.c`. The destination is a fresh path-preserving
translation of every selected constant and expression. `issecure_mask(X)` is
an exported function-like Rust macro so downstream translations can invoke the
same source-visible interface; its expansion uses one explicit `i32` left
operand and evaluates the caller expression once, preserving the selected
C `int` mask expressions. No compiler, formatter, linker, test, runtime, or
historical Lupos source was used.
