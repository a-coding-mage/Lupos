# Parity review — S000772 / attempt 1 / slot 1

Reviewed `src/arch/x86/include/uapi/asm/debugreg.rs` against pinned
`vendor/linux/arch/x86/include/uapi/asm/debugreg.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, with the frozen x86_64 task,
symbol, scope, ABI, lifetime, and branding context.

Result: **APPROVE**.

Evidence:

- Linux symbols `DR_FIRSTADDR`, `DR_LASTADDR`, `DR_STATUS`, and `DR_CONTROL`
  retain their exact integer values and signed C `int` representation as Rust
  `i32` constants.
- Linux symbols `DR6_RESERVED`, `DR_TRAP0` through `DR_TRAP3`,
  `DR_TRAP_BITS`, `DR_BUS_LOCK`, `DR_STEP`, and `DR_SWITCH` retain the exact
  masks and values.  `DR6_RESERVED` is correctly an unsigned 32-bit constant,
  matching the C hexadecimal literal's `unsigned int` type; the remaining
  unsuffixed literals fit in and retain C `int` semantics.
- Linux symbols `DR_CONTROL_SHIFT`, `DR_CONTROL_SIZE`, `DR_RW_EXECUTE`,
  `DR_RW_WRITE`, `DR_RW_READ`, `DR_LEN_1`, `DR_LEN_2`, `DR_LEN_4`, and
  `DR_LEN_8` retain their values and `i32` expression type.  The composed
  `DR_TRAP_BITS` expression preserves the four-bit OR value.
- Linux symbols `DR_LOCAL_ENABLE_SHIFT`, `DR_GLOBAL_ENABLE_SHIFT`,
  `DR_LOCAL_ENABLE`, `DR_GLOBAL_ENABLE`, `DR_ENABLE_SIZE`,
  `DR_LOCAL_ENABLE_MASK`, and `DR_GLOBAL_ENABLE_MASK` retain the exact
  x86 mask and shift encodings.
- Linux symbol `DR_CONTROL_RESERVED` follows the selected `#else` branch:
  frozen task architecture is x86_64, not `__i386__`; the Rust `u64` value
  `0xFFFF_FFFF_0000_FC00` matches the Linux `unsigned long` literal
  `0xFFFFFFFF0000FC00UL` on x86_64.  The excluded i386 `0xFC00` branch is not
  in this task's architecture scope.
- Linux symbols `DR_LOCAL_SLOWDOWN` and `DR_GLOBAL_SLOWDOWN` retain their
  exact values.  Direct pinned callers use these constants in the expected
  shifts, masks, and unsigned-long expressions (for example,
  `__encode_dr7()` and `ptrace_write_dr7()`); the candidate has not altered
  any caller mechanism, ABI, ordering, allocation, locking, or error path.
- The header contains no functions, data objects, layouts, linkage, locking,
  allocation, refcount, RCU, cleanup, or error-path behavior to translate.
  No unauthorized branding appears; the SPDX identifier and immutable
  provenance identify the pinned source, revision, x86_64 scope, and task.

No SC1 findings.
