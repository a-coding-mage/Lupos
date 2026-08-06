# Parity review — S000713 (slot 1)

## Scope and inputs

- Task row verified on `feat/bun-like-rewrite-test`: `S000713`, P02, attempt 1, `REVIEWING`, source `arch/x86/include/asm/syscalls.h`, destination `src/arch/x86/include/asm/syscalls.rs`, architecture `x86_64`.
- Pinned Linux revision verified against `vendor/linux.SHA`: `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Reviewed the complete pinned header (SHA-256 `8655e80fcb5895893bee11f1c6cd3ce58117d4a9bc650e2a9777faa8f1567b31`) against the candidate (SHA-256 `ce98f7fac35fb25f39aeb41ed90ea0a53fcfef73d57966fd52d6b0c43e8eeaa1`), the S000713 scope/symbol records, frozen x86_64 configuration, header-closure evidence, and the defining/consuming `arch/x86/kernel/ioport.c` context.

## Result

No parity findings.

The only operative declaration in the Linux header, `ksys_ioperm(unsigned long, unsigned long, int) -> long`, is represented exactly once as an unsafe C-ABI declaration with the same unmangled symbol name and parameter/return order. For the frozen `x86_64` task, `c_ulong`, `c_int`, and `c_long` respectively preserve the two unsigned-long arguments, the signed `int` argument, and the signed `long` return type. `ioport.c` confirms that `ksys_ioperm` has external linkage and this exact signature in both conditional definition branches; the frozen configuration enables `CONFIG_X86_IOPL_IOPERM`.

The header guard has no runtime/ABI payload and is correctly not represented as a separate Rust declaration. There are no further function, type, macro, conditional, linkage, or configuration-selected declarations to translate. The candidate provenance lines match the task, source path, frozen revision, and `x86_64` architecture.

## Review constraints

Source inspection only. No compiler, formatter, rust-analyzer, build, test, debugger, or runtime command was used. No candidate source or queue state was changed.
