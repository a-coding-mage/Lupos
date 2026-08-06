# S000713 implementation record

Translated `arch/x86/include/asm/syscalls.h` from pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` to the leased destination
`src/arch/x86/include/asm/syscalls.rs`.

The complete source has one unconditional declaration:
`long ksys_ioperm(unsigned long from, unsigned long num, int turn_on);`.
The destination preserves it as an external `C` symbol with `c_long`, two
`c_ulong` arguments, and a `c_int` argument. The frozen x86_64 consumer command
for `arch/x86/kernel/ioport.o` contains `--target=x86_64-linux-gnu` and `-m64`.
No wrapper, implementation, feature conditional, or syscall facade was added.

Source context inspected: the full pinned header; both `ksys_ioperm` branches
and syscall caller in `arch/x86/kernel/ioport.c`; `arch/x86/kernel/Makefile`;
the frozen x86_64 config (`CONFIG_X86_64=y`, `CONFIG_X86_IOPL_IOPERM=y`);
the header-closure and compile-command facts; and relevant syscall declarations
in `include/linux/syscalls.h` and `arch/x86/include/asm/unistd.h`.
