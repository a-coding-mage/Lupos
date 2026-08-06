# Implementation — S000772

Translated `arch/x86/include/uapi/asm/debugreg.h` to
`src/arch/x86/include/uapi/asm/debugreg.rs` for the frozen x86_64 subset.

- Preserved every selected UAPI debug-register macro as a public Rust
  constant, including the computed `DR_TRAP_BITS` expression.
- Kept C integer-literal categories: the unsuffixed small constants are
  `i32`, `DR6_RESERVED` is the x86_64 `unsigned int` value (`u32`), and the
  `UL`-suffixed x86_64 `DR_CONTROL_RESERVED` is `core::ffi::c_ulong`.
- Selected the `#else` branch of `DR_CONTROL_RESERVED`, because this task's
  frozen architecture is x86_64; the `__i386__` branch is outside this task's
  architecture membership.
- The C include guard has no Rust declaration equivalent.  This header
  declares no types, storage, functions, or ABI layouts.

Source and consumer context examined: the complete pinned
`vendor/linux/arch/x86/include/uapi/asm/debugreg.h`, plus its x86 consumers in
`arch/x86/kernel/kgdb.c`, `arch/x86/kernel/hw_breakpoint.c`,
`arch/x86/kernel/ptrace.c`, and `arch/x86/include/asm/traps.h`.
