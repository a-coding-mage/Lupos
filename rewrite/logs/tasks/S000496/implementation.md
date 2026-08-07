Task S000496 implements the pinned x86 CPU feature header at
`arch/x86/include/asm/cpufeatures.h` (Linux revision
425f94c2954b1fe80ebdbf9b29854e89750355df) for x86_64.

The fresh Rust source preserves the complete feature-word and bug-bit constant
set, arithmetic expressions, comments used for cpuinfo naming, and the
CONFIG_X86_32 conditional ESPFIX definition. The function-like X86_BUG macro is
represented as a const function with the same NCAPINTS*32 offset. No source
compiler or formatter was invoked.
