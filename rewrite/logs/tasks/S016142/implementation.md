# Implementation — S016142

Translated `include/uapi/linux/handshake.h` at pinned revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` to
`src/include/uapi/linux/handshake.rs`.

- Preserved the three named C enum tags as transparent `c_int` wrappers and
  every enumerator value, including the terminal handler-class sentinel.
- Preserved every anonymous netlink attribute/command enum as `c_int`
  constants, including each derived `*_MAX` expression.
- Preserved all three C string-literal macro values as NUL-terminated
  `c_char` static arrays, retaining C static storage and array-to-pointer decay
  through `.as_ptr()`.
- The source header is unconditional for the frozen common x86_64/AArch64
  union. Its C include guard has no Rust runtime or ABI equivalent.

No tests, drivers, module indexes, formatting, compilation, or runtime actions
were added or run.
