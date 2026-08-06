# Rust review — S000803 (attempt 2)

Reviewed `vendor/linux/arch/x86/include/uapi/asm/unistd.h` at pinned revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/arch/x86/include/uapi/asm/unistd.rs`, the S000803 frozen symbol inventory,
the x86_64 frozen configuration, and the recorded generated-header/Kbuild
context. This was a source-only review; no compiler, formatter, linker, test,
or Rust-analyzer diagnostics were used.

## Findings

1. **HIGH — the non-kernel public-header conditional surface is omitted.**
   Upstream lines 15–23 select and include `asm/unistd_32.h` for `__i386__`,
   `asm/unistd_x32.h` for `__ILP32__`, and `asm/unistd_64.h` otherwise. The
   candidate exposes only `__X32_SYSCALL_BIT`; it has no representation of any
   of those three branches or their syscall-number macro surface. The frozen
   inventory explicitly lists these conditionals (S000803 `ifndef@15`,
   `ifdef@16`, `elif@18`, `else@20`, and matching `endif`s), and upstream
   `arch/x86/include/uapi/asm/Kbuild` plus
   `arch/x86/entry/syscalls/Makefile:27–42,65` records all three generated
   public headers. `unistd_32.h` and `unistd_64.h` are materialized Phase 0
   BUILD_METADATA dependencies; the absence of a Rust task for generated
   payloads does not make the public conditional behavior disappear. Although
   `__KERNEL__` makes this block inactive for the frozen kernel compile command,
   the source file is UAPI and its inactive public branch remains part of the
   selected file's declared conditional surface. The applier must establish
   and implement the path-preserving Rust representation/ownership of these
   public generated interfaces, or block the task rather than close it as a
   one-constant translation.

2. **HIGH — `i32` preserves the standalone literal type but not the C macro's
   promotion behavior, and the candidate's public explanation implies more
   than Rust provides.** In C, `0x40000000` has type `int`; it is an untyped
   preprocessor replacement token sequence and participates in the C usual
   arithmetic conversions at each use. For example, upstream expressly
   documents `nr & ~__X32_SYSCALL_BIT` in lines 5–11. The Rust `i32` constant
   correctly represents the literal's standalone signed 32-bit value, but it
   is a typed item: `!__X32_SYSCALL_BIT` remains `i32` and cannot be combined
   with a `u32`, `u64`, or `usize` syscall-number expression without an
   explicit conversion at the consuming operation. Consequently the prose
   claim that user-space expressions retain the intended C behavior is not
   satisfied by this item alone. The final mapping needs an explicit,
   source-backed rule for every Rust-facing syscall-number width/use (including
   the generated public header branches in finding 1), preserving C conversion
   order and bit pattern rather than relying on implicit Rust typing.

3. **HIGH — SPDX provenance was changed.** The pinned UAPI source begins
   `SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note`; the candidate
   changes it to `GPL-2.0-only`. This is neither an allowed branding change nor
   faithful retention of the source identifier. Restore the exact upstream
   SPDX expression before acceptance.

## Result

Rejected pending applier resolution of all three findings. No ownership,
`unsafe`, layout, allocation, panic, or drop behavior is present in this
candidate beyond the public constant/interface defects above.
