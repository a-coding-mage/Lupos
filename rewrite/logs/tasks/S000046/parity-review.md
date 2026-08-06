# S000046 parity review — slot 1

Reviewed candidate: `src/arch/arm64/include/asm/compiler.rs`.

Reviewed oracle: `vendor/linux/arch/arm64/include/asm/compiler.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, with the frozen AArch64 config,
metadata, and retained Kbuild command context.

## Finding P1 — the selected inline-assembly template is not represented as a
usable Rust assembly template

The selected C expansion of `ARM64_ASM_PREAMBLE` is the preprocessor string
literal fragment `".arch armv8.5-a\\n"`.  `xpaclri` concatenates that fragment
with its `"\\thint\\t#7\\n"` template before Clang parses the inline assembly
(`vendor/linux/arch/arm64/include/asm/compiler.h:5-18`).  The same fragment is
also concatenated by `__TLBI_0`/`__TLBI_1` and by `__MTE_PREAMBLE`
(`vendor/linux/arch/arm64/include/asm/tlbflush.h:32-37` and
`vendor/linux/arch/arm64/include/asm/mte-def.h:16`), with active MTE support in
the frozen configuration.

The candidate changes this into the expression macro
`arm64_asm_preamble!()` at lines 13-17 and then supplies the *nested macro
invocation* itself as the first `core::arch::asm!` template argument at line
34.  That is not the C preprocessor's adjacent-string-literal concatenation,
and it does not provide a composable Rust template fragment for the other
selected inline-assembly users.  In particular, Rust has no C-style adjacent
string-literal concatenation through which a later translated `__MTE_PREAMBLE`
or `__TLBI_*` analogue can append its instruction text.  The candidate's own
`xpaclri!` therefore does not carry the exact selected `".arch armv8.5-a\\n\\thint\\t#7\\n"`
assembly template in a form that can be parsed and emitted as one inline-asm
template.

Required resolution: preserve the frozen `.arch armv8.5-a` directive and the
`hint #7` instruction as a valid, single Rust inline-assembly template at the
`xpaclri` call site, and supply a design that preserves the selected preamble's
template-composition role for the translated TLBI/MTE users.  Do not replace the
directive with a runtime string or drop it based on the current assembler.

## Confirmed mappings

- Every frozen AArch64 C translation-unit command defines
  `ARM64_ASM_ARCH=\"armv8.5-a\"`, targets `aarch64-linux-gnu`, uses the LLVM
  integrated assembler, and carries `-Wa,-march=armv8.5-a`; hard-coding that
  selected architecture value is therefore correct for this frozen subset.
- `CONFIG_ARM64_PTR_AUTH=y` and `CONFIG_ARM64_PTR_AUTH_KERNEL=y`; the
  candidate correctly selects `xpaclri!` for both
  `ptrauth_strip_{kernel,user}_insn_pac` macros.
- `CONFIG_BUILTIN_RETURN_ADDRESS_STRIPS_PAC=y`; the C `#if !defined(...)`
  wrapper is inactive, so the candidate correctly introduces no replacement
  return-address macro.
- Apart from P1, the candidate preserves the operative `xpaclri` structure:
  one operand evaluation, an AArch64-width unsigned result, explicit x30
  read/write operand, `hint #7`, and no memory or stack side effect.  The
  pinned direct consumers are `arch/arm64/kernel/process.c` and
  `arch/arm64/kernel/stacktrace.c`.

No build, compiler, formatter, test, or runtime command was run.
