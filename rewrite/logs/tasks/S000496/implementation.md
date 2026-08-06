# S000496 implementation

Translated `arch/x86/include/asm/cpufeatures.h` for the frozen x86_64 configuration and Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.

The destination preserves the source registry's 32-bit capability/erratum index representation: every selected object-like feature and bug macro is a public `u32` constant retaining its original `word * 32 + bit` expression. `NCAPINTS` and `NBUGINTS` retain their source values, and the parameterized `X86_BUG(x)` macro is represented by an uppercase `const fn` with the same `NCAPINTS * 32 + x` arithmetic.

`X86_BUG_ESPFIX` is deliberately absent: its sole upstream definition is guarded by `CONFIG_X86_32`, which is not part of this x86_64 task's frozen configuration. The header guard and preprocessor directives have no Rust runtime representation.

No source outside the leased destination and this task evidence was edited. No build, formatter, compiler, test, or runtime command was run.
