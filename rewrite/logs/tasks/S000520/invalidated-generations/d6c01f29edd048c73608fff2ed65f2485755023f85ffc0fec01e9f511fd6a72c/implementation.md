# S000520 implementation

Source and task identity were verified on `feat/bun-like-rewrite-test` against
`vendor/linux.SHA` revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.

`arch/x86/include/asm/emulate_prefix.h` is selected for x86_64 header closure
with two Rust consumers.  It contains only two comma-separated initializer
macros: the Xen and KVM five-byte instruction-emulation escape sequences.
The Rust destination represents each macro's resulting byte sequence as a
public `[u8; 5]` constant.  The byte order and values are unchanged:
UD2 (`0x0f, 0x0b`) followed by the ASCII signature (`xen` or `kvm`).

No types, layouts, linkage, allocation, synchronization, cleanup, feature
conditionals, or executable decoder behavior are present in this header.
The source-level `PENDING_REVIEW` macro records require final reviewer/applier
closure; no manifest was edited here.
