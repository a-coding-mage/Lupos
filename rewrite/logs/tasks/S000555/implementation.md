# S000555 implementation

Source: `vendor/linux/arch/x86/include/asm/inat_types.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The complete selected header has no includes, configuration branches, storage,
functions, enums, bitfields, unions, or structures.  Its include guard has no
Rust runtime or type-system equivalent.  The three typedefs are preserved as
public aliases with the x86_64 C scalar widths and signedness: `unsigned int`
is `u32`, `unsigned char` is `u8`, and `signed int` is `i32`.

Consumer context was read in `asm/inat.h`, `asm/insn.h`, `arch/x86/lib/inat.c`,
and `arch/x86/lib/insn-eval.c`: the aliases carry instruction-attribute words,
decoded instruction bytes, and signed decoded field values respectively.  The
frozen x86_64 configuration selects `CONFIG_X86_64=y` and `CONFIG_X86=y`; the
scope and header-closure records classify this exact header as x86_64
`RUST_TRANSLATE`.
