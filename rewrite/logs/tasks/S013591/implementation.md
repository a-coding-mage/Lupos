# Implementation evidence

- Task: `S013591`
- Attempt: `1`
- Pipeline: `P01`
- Linux source: `vendor/linux/include/linux/circ_buf.h`
- Destination: `src/include/linux/circ_buf.rs`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architectures: `common` (selected by x86_64 and AArch64)

Read the complete pinned header and its frozen symbol, ABI, lifetime, scope, and
header-closure records. The translation preserves `struct circ_buf` as a C
layout type with a mutable character pointer and two C `int` fields. Each
function-like macro remains an exported Rust macro so caller expressions retain
macro expansion and evaluation order. `CIRC_CNT_TO_END` and
`CIRC_SPACE_TO_END` retain the Linux statement-expression locals; operands that
Linux evaluates more than once remain expanded more than once. Wrapping
arithmetic is explicit at each C arithmetic operation to preserve ring-index
wraparound.

No compiler, formatter, linker, test, runtime, or historical Lupos source was
used.
