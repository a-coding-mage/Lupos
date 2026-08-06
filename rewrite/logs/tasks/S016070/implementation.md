# S016070 implementation

Translated `include/uapi/linux/bpf_common.h` to the path-preserving
`src/include/uapi/linux/bpf_common.rs` from pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`; the frozen task records it as
`common` for the x86_64/AArch64 union.

The translation retains every selected BPF instruction class, field mask,
opcode, source selector, and `BPF_MAXINSNS` value. The five function-like C
macros are represented by public generic functions whose sealed input trait
models C integer promotions and the usual arithmetic conversions for the
frozen x86_64 and AArch64 LP64 targets.

No build, formatting, test, or runtime command was run.
