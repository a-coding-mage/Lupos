# Implementation — S016234 attempt 2

Translated `include/uapi/linux/major.h` from pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` to
`src/include/uapi/linux/major.rs`.

The C include guard is intentionally represented by no Rust item. Each of the
139 C device-major value macros is a public `i32` Rust constant with its UAPI
identifier preserved. `HD_MAJOR` preserves its named alias and
`UNIX98_PTY_SLAVE_MAJOR` preserves the named addition expression. The constants
are common to the frozen x86_64 and AArch64 configurations. No layout,
ownership, locking, ABI, or unsafe behavior is present in this header.
