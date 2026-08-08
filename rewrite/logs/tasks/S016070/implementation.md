# Implementation — S016070

Source: `vendor/linux/include/uapi/linux/bpf_common.h`
Destination: `src/include/uapi/linux/bpf_common.rs`
Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
Architectures: `common`
Task attempt: `P02/a1`

The fresh Rust file preserves the complete selected header surface: all BPF
instruction class, load/store size and mode, ALU/jump operation, and source
constants, the four masked extraction macros as `const fn` operations, and
`BPF_MAXINSNS` set to 4096. No tests, compiler, formatter, or runtime command
was used.
