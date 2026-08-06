# Implementation — S000758

Source: `vendor/linux/arch/x86/include/asm/vmxfeatures.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The source contains `NVMXINTS` and 64 object-like VMX feature-index macros.
The candidate maps each to a public `u32` constant with the original
`word * 32 + bit` expression, retaining all sparse bit positions and the
three source control-word groupings.  The original header has no conditional
compilation, functions, types, storage, locking, allocation, ABI declarations,
or ownership/lifetime behavior.

No build, format, test, or runtime command was run.
