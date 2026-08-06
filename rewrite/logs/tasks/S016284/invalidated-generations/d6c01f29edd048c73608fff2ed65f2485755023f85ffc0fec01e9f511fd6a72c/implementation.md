# Implementation — S016284

Translated `include/uapi/linux/netfilter/xt_LOG.h` to
`src/include/uapi/linux/netfilter/xt_LOG.rs` from pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The selected UAPI surface contains seven unsuffixed C `int` object-like
macros and `struct xt_log_info`.  The macros are `c_int` constants with their
exact values.  `xt_log_info` uses `#[repr(C)]`; its two `unsigned char` fields
are `u8`, and its 30-byte `char` array is `u8` because the frozen x86_64 and
AArch64 Kbuild commands both select `-funsigned-char`.  This preserves the
32-byte, byte-aligned C layout used as `xt_target.targetsize` by `xt_LOG.c`.

No configuration conditional changes this header for the frozen common scope.
No compiler, formatter, linker, test, or historical Rust source was used.
