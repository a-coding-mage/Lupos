# Implementation — S000730

Translated `arch/x86/include/asm/trapnr.h` to
`src/arch/x86/include/asm/trapnr.rs` from pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` for the frozen x86_64 scope.

- Mapped all eight FRED/Intel VT-x/AMD SVM event-type macros as public `i32`
  constants with their original values `0` through `7`.
- Mapped all twenty-four interrupt/exception trap-number macros as public
  `i32` constants, retaining the non-contiguous `X86_TRAP_VC = 29` and
  `X86_TRAP_IRET = 32` values.
- The macros are unsuffixed decimal C integer literals.  On frozen x86_64
  these have signed `int` semantics; `i32` preserves their source width and
  signedness before use-site C conversion rules are applied.
- The include guard has no Rust declaration or ABI counterpart.  The header
  has no includes, functions, types, storage, configuration branches, locking,
  allocation, or cleanup behavior.

Context examined: the complete source header; frozen x86_64 configuration
(`CONFIG_X86_64=y`, `CONFIG_X86_FRED` disabled); header-closure and include-edge
records; and consumers in `asm/fred.h`, `asm/idtentry.h`, `asm/vmx.h`,
`arch/x86/entry/common.c`, and `arch/x86/entry/entry_fred.c`.  The constants
remain available even where a frozen configuration disables a particular
consumer feature, matching the unconditional source declarations.
