# Implementation — S013801 attempt 2

Translated `include/linux/dqblk_v1.h` from pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` to
`src/include/linux/dqblk_v1.rs`.

The source contains four object-like, unadorned decimal integer macros. Each
is represented as a public `core::ffi::c_int` constant, preserving the C
`int` expression type and exact values: `1`, `1`, `0`, and `2`.

The header has no declarations, layout-bearing types, conditional selected
branches, ownership rules, locking, or ABI linkage beyond these integer macro
expressions. It is selected for the frozen common x86_64/AArch64 scope.
