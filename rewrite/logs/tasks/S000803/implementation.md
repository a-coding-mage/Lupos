# S000803 implementation, attempt 2

- Task/destination: `S000803` / `src/arch/x86/include/uapi/asm/unistd.rs`
- Oracle: `vendor/linux/arch/x86/include/uapi/asm/unistd.h` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Frozen selection: x86_64 kernel configuration (`CONFIG_64BIT=y`,
  `CONFIG_X86_64=y`).  The header is a selected translation header in the
  materialized x86_64 header closure.

The active kernel-side definition is translated exactly as the public `i32`
constant `__X32_SYSCALL_BIT = 0x4000_0000`.  `i32` preserves the upstream
macro's C `int` type.  The `!__KERNEL__` alternatives only select user-UAPI
generated syscall-number headers and are inactive for this frozen kernel
translation task; no syscall-number definitions belong to this source file's
active kernel branch.

No compiler, formatter, build, test, or historical Lupos source was used.
