# S016105 implementation

- Task: `S016105`; pipeline: `P01`; attempt: `1`.
- Source: `vendor/linux/include/uapi/linux/dpll.h` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Destination: `src/include/uapi/linux/dpll.rs`.
- Scope is common to x86_64 and aarch64, with the header selected through
  `net/core/rtnetlink.o` for both frozen configurations.
- Translated every operative macro and every C enum as a C-`int` type alias
  plus value-preserving constants. Each C implicit enumerator has its source
  integer value made explicit; private maximum sentinels and public maximum
  aliases retain their C expressions.
- No conditional configuration branch affects the header body. No storage,
  ownership, locking, allocation, or cleanup operation is present.

The source was reviewed directly in full. No compilation, formatting, testing,
or diagnostic command was run.
