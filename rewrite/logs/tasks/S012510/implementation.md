# Implementation: S012510

Source: `include/asm-generic/bitops/builtin-ffs.h` at Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The selected AArch64 header defines only `ffs(x)` as `__builtin_ffs(x)`.
The compiler builtin has the C `int` argument and result contract, so the Rust
surface is `ffs(i32) -> i32`.  The result is `0` for zero and otherwise the
one-based position of the least-significant set bit.  This is corroborated by
the pinned generic fallback in `include/asm-generic/bitops/ffs.h` and by the
selected ARM64 callers in `kernel/setup.c` and `kvm/vgic/*`, which explicitly
handle the zero result before using `ffs(x) - 1`.

`i32::trailing_zeros` queries the same signed-`int` bit representation; the
explicit zero branch preserves the builtin's defined `ffs(0) == 0` behavior
rather than Rust's zero-count result of 32.  The header has no storage,
linkage, FFI layout, synchronization, ownership, or cleanup contract.

No compiler, formatter, linker, test, debugger, or historical Lupos Rust
source was used.
