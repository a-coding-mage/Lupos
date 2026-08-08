# S016142 implementation

- Task: `S016142`
- Pipeline: `P02`
- Linux source: `vendor/linux/include/uapi/linux/handshake.h`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Destination: `src/include/uapi/linux/handshake.rs`
- Architectures: `x86_64,aarch64` (`common`)
- Source SHA-256: `a6c0c5e66d6afcf7527e9e23c6facc30897cda056759e314318eaae2e4b61304`

The complete 76-line pinned UAPI header was read. Its two string macros and
version macro are represented as public constants; the three named C enums are
represented as `#[repr(C)]` enums with their explicit zero-based values. The
four anonymous C enums are represented by public `i32` constants, retaining
the Linux `__MAX` intermediate constants and subtract-one maximum expressions.
The UAPI C enum underlying ABI is `int`; `repr(C)` and `i32` preserve that
width for both approved architectures. Include guards and generator comments
have no Rust runtime/API equivalent. No conditional branches are selected by
the frozen configurations beyond the unconditional header body.

No unsafe code, allocation, locking, lifetime-sensitive state, callers, or
callees are present in this header. `candidate.diff` is the fresh destination
snapshot.
