# S013591 implementation

Translated `include/linux/circ_buf.h` (the common header selected by both
frozen architectures) from pinned revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` to
`src/include/linux/circ_buf.rs`.

The `circ_buf` ABI remains a C-layout raw byte pointer followed by two C
`int` counters. The frozen x86_64 and AArch64 compile commands specify
`-funsigned-char`, so the pointee is represented as `u8` without changing its
pointer ABI. It owns neither the allocation nor synchronization.

All four upstream expression macros remain exported expression macros. They
bind each supplied operand once; the two `_TO_END` forms retain the source
GNU statement-expression `int` temporaries and their comparison/selection.
Target-width wrapping operations make the intended kernel counter arithmetic
independent of Rust debug overflow checks. The macro interface requires the
same compatible counter operands as the translated C call site, including the
selected `unsigned long` ring-buffer use.

No build, formatting, test, or runtime command was run.
