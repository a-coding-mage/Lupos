# S000803 implementation

- Lease: `P02` / `codex-root-cont2-20260806-p02`; attempt 1.
- Oracle: `vendor/linux/arch/x86/include/uapi/asm/unistd.h` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Frozen scope: x86_64, `RUST_TRANSLATE`, destination
  `src/arch/x86/include/uapi/asm/unistd.rs`.

The active kernel-view content of the header is the unconditional
`__X32_SYSCALL_BIT` macro.  It is represented as an explicitly typed `i32`,
matching the C integer-literal type.  The `!__KERNEL__` include selection for
`unistd_32.h`, `unistd_x32.h`, and `unistd_64.h` is inactive for the selected
kernel compile context and therefore has no Rust counterpart in this task.
Those headers are Kbuild-generated user-UAPI syscall-number headers; their
generation is recorded in `arch/x86/entry/syscalls/Makefile`.

No compiler, formatter, test, runtime tool, or historical Lupos Rust source
was used.
