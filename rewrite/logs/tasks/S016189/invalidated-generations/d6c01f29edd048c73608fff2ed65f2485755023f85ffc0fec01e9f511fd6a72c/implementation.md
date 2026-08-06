# Implementation evidence — S016189

Fresh translation of `include/uapi/linux/input-event-codes.h` into `src/include/uapi/linux/input-event-codes.rs`.

- Pinned Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Scope/queue: `S016189`, common x86_64/aarch64 UAPI header, P02 attempt 1.
- The C header contains only an include guard and 795 object-like UAPI value/alias macros; the guard has no Rust runtime equivalent.
- All 795 macros are represented in source order as public `u32` constants, retaining numeric expressions and alias relationships verbatim.
- No C structures, enums, function-like macros, conditional configuration branches, or ABI layouts occur in this header.
- Source-level inventory comparison confirmed the complete macro name/value sequence matches the pinned header.

