# Parity review — S000071

Reviewed `vendor/linux/arch/arm64/include/asm/gpr-num.h` at pinned revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/arch/arm64/include/asm/gpr-num.rs`, the S000071 scope/symbol records, and
direct AArch64 uses.  Result: **reject; source parity is not established.**

## Findings

1. **Missing `__ASSEMBLER__` branch (blocking).**  The Linux header's true
   branch emits unquoted assembler directives at inclusion time: an `.irp` over
   0 through 30, `.equ` definitions for both `x` and `w` register spellings,
   `.endr`, and the two zero-register definitions.  The candidate has no
   representation of that branch or its conditional selection; it only defines
   a Rust string value.  A Rust constant is not emitted merely by including a
   Rust module in assembly, so it cannot provide the assembler-header behavior.
   This leaves the selected `ifdef@5` branch in `SYMBOLS.tsv` untranslated.

2. **`__DEFINE_ASM_GPR_NUMS` macro expansion semantics are not preserved
   (blocking).**  Linux exposes a function-like-in-use object-like
   preprocessor macro whose replacement tokens are adjacent C string literals.
   It is expanded directly inside inline-assembly template construction, then
   composed with surrounding literals/macros before the assembler sees the
   resulting directives.  Replacing it with `pub const ...: &str` changes that
   interface into a Rust item and supplies no macro-expansion or template
   composition mechanism.  Direct upstream consumers demonstrate the required
   behavior: `arch/arm64/kvm/pauth.c:PACGA`, `asm/sysreg.h:DEFINE_MRS_S` and
   `DEFINE_MSR_S`, `asm/asm-extable.h`'s extable macros, and the two inline-asm
   sites in `asm/fpsimd.h` all place the macro immediately beside additional
   assembly template text.  The candidate neither preserves that source-token
   interface nor provides an equivalent integration at each such use.

3. **The candidate's literal is only a correct fragment, not a complete
   translation.**  Its decoded `&str` payload does retain the C branch's
   directive order, tabs, backslash-`num` escapes, register range, and terminal
   newlines.  That is necessary for the non-assembler macro result, but it does
   not cure findings 1–2: text held in a Rust constant is not the C
   preprocessor expansion and does not make the `.equ` definitions available
   to either GNU-style inline assembly construction or assembler preprocessing.

## Required resolution

The applier must supply a path-preserving representation that preserves both
conditional branches and the exact expansion-at-use-site contract for all
direct ARM64 consumers, including their directive tokens and newline behavior.
It must not accept this `&str` as an equivalent macro interface without
source-level evidence of how Rust inline/global assembly consumers receive the
same composed template and how assembler consumers receive the unquoted branch.
