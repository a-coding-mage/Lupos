# Implementation — S000749

Source: `vendor/linux/arch/x86/include/asm/vermagic.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The frozen x86_64 configuration sets `CONFIG_X86_64=y`.  Its selected header
branch deliberately does not define `MODULE_PROC_FAMILY`; the following
`CONFIG_X86_32` false branch defines `MODULE_ARCH_VERMAGIC` as the empty string
literal.  The other processor-family alternatives depend on x86_32 and are not
selected for this task's only architecture.

`kernel/module/main.c` defines `INCLUDE_VERMAGIC`, includes
`linux/vermagic.h`, and initializes `vermagic` with `VERMAGIC_STRING`.
`linux/vermagic.h` composes that string from preprocessor string tokens,
including `MODULE_ARCH_VERMAGIC`.  The Rust translation therefore supplies an
exported `MODULE_ARCH_VERMAGIC!()` macro whose sole x86_64 expansion is the
same empty literal, so the future Rust consumer can retain compile-time string
composition rather than using a runtime value.

Rust module inclusion replaces the C include guard.  No storage, ABI object,
or processor-family symbol is introduced.
