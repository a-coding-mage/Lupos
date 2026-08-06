# S000779 implementation

Translated `arch/x86/include/uapi/asm/ldt.h` to
`src/arch/x86/include/uapi/asm/ldt.rs` for the frozen x86_64 configuration and
Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.

The header's three integer macros retain C `int`-compatible `i32` values. The
`user_desc` representation is `#[repr(C)]` with three `u32` members followed by
one `u32` bit-field allocation unit: x86_64 GNU C allocates the seven declared
bit-fields, including `lm`, in bits 0 through 7 of that final 32-bit unit.
The remaining 24 bits are retained, not synthesized or discarded. Accessors
and setters use the source-order masks and setters preserve every other bit.

Reviewed pinned context: the complete UAPI header; `arch/x86/kernel/tls.c`; and
`arch/x86/include/asm/desc.h`, which consume the named fields and establish the
same ordering. The frozen configuration selects `CONFIG_X86_64=y` and
`CONFIG_MODIFY_LDT_SYSCALL=y`.

No compiler, formatter, build, or test command was run.
