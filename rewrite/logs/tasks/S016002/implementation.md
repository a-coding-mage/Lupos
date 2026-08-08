# Implementation evidence

- Task: S016002, attempt 1, pipeline P02
- Source: `vendor/linux/include/uapi/asm-generic/errno-base.h`
- Destination: `src/include/uapi/asm-generic/errno-base.rs`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architectures: `common`
- Source review: read the complete pinned header. It contains only the SPDX
  notice, an include guard, and the 34 base errno object-like macros.
- Translation: each errno macro is represented as a public `i32` constant with
  the exact Linux identifier and integer value. The include guard is represented
  by the Rust module boundary; no conditional branch changes a value.
- No compiler, formatter, build, test, runtime, or historical-source access was
  used.
