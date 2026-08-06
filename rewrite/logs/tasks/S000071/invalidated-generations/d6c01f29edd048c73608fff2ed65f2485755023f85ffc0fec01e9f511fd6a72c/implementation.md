# Implementation — S000071

Oracle: `vendor/linux/arch/arm64/include/asm/gpr-num.h` at Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.

The source has no C data, functions, or ABI objects.  Its operative C-side macro is represented as the exact assembly string constant `__DEFINE_ASM_GPR_NUMS`.  It emits the `.irp` mappings for `x0` through `x30` and `w0` through `w30`, then the explicit `xzr` and `wzr` mappings to 31.  This matches its use by ARM64 inline-assembly emitters such as `asm-extable.h`, `sysreg.h`, `fpsimd.h`, and `kvm/pauth.c`.

The `__ASSEMBLER__` branch carries the same assembler directives directly; no Rust runtime behavior or layout is introduced.

No compiler, formatter, linker, test, or runtime command was run.
