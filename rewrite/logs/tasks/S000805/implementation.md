# S000805 implementation

Implemented `arch/x86/include/uapi/asm/vmx.h` as the fresh, path-preserving
`src/arch/x86/include/uapi/asm/vmx.rs` translation for x86_64.

The source contains every numeric VMX exit-reason and abort macro with the C
literal's x86_64 integer category preserved (`u32` for the out-of-range
unsigned `0x80000000`, `i32` for the remaining unsuffixed integer literals).
`VMX_EXIT_REASONS` and `VMX_EXIT_REASON_FLAGS` remain expansion-time macros;
their expansions produce the source-ordered `(reason, name)` entries and do
not omit the C list's deliberately absent reason values.  The include guard
has no Rust runtime analogue and is represented by the Rust module boundary.

Source evidence: `vendor/linux/arch/x86/include/uapi/asm/vmx.h:29-173` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`; selection evidence is
`rewrite/metadata/header_closure.tsv` via the S000805 scope row.

No compiler, formatter, linker, test, or runtime command was used.
