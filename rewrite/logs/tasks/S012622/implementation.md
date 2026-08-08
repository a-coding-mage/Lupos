Task S012622, attempt 1, pipeline P02.

Source: vendor/linux/include/crypto/ecdh.h at Linux revision
425f94c2954b1fe80ebdbf9b29854e89750355df; destination:
src/include/crypto/ecdh.rs; architecture: aarch64.

The complete pinned header was read. It contains the include guard, four curve
ID macros, `struct ecdh`, and three C function declarations. The destination
preserves the C ABI with `#[repr(C)]`, fixed-width integer types, raw pointers,
and `extern "C"` declarations. The decode declaration retains Linux's aliasing
contract: the resulting key pointer refers into the input packet buffer.

The selected header has no executable branches beyond the include guard. No
compiler, formatter, linker, test, runtime, or historical Lupos source was
used.
