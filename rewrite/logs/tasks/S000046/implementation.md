# S000046 implementation record

Source: vendor/linux/arch/arm64/include/asm/compiler.h at
425f94c2954b1fe80ebdbf9b29854e89750355df.

The queue lease is P02 / p02-terra-fallback, and the frozen scope maps this
AArch64 header one-to-one to src/arch/arm64/include/asm/compiler.rs.

Frozen mechanical inputs:

- rewrite/configs/aarch64/frozen.config sets
  CONFIG_ARM64_PTR_AUTH=y, CONFIG_ARM64_PTR_AUTH_KERNEL=y, and
  CONFIG_BUILTIN_RETURN_ADDRESS_STRIPS_PAC=y.
- The retained AArch64 Kbuild command defines ARM64_ASM_ARCH="armv8.5-a" with
  the LLVM 19 compiler at /usr/lib/llvm-19/bin/clang, target
  aarch64-linux-gnu, and LLVM_IAS=1.
- COMPILER_PREDICATES.tsv contains no direct predicate record for this header.
  The return-address behavior is instead mechanically selected by the frozen
  Kconfig result above; its active C branch leaves the compiler builtin
  unwrapped.

Mapping:

- ARM64_ASM_PREAMBLE becomes an exported token macro yielding the exact
  .arch armv8.5-a directive and newline, not a runtime string value.
- xpaclri(ptr) remains an expression macro. It evaluates its operand once,
  binds that value to x30, executes hint #7, and yields the resulting usize.
  The nomem, nostack, and preserves_flags options mirror the C
  extended-assembly constraints, which name only the read/write register.
- Both active pointer-auth stripping macros expand to xpaclri!; their disabled
  identity alternatives are not selected by the frozen configuration.
- The inactive __builtin_return_address wrapper is deliberately absent: the
  selected C preprocessor branch preserves the compiler builtin rather than
  defining a replacement.

No compiler, formatter, linker, test, runtime, or benchmark command was run.
