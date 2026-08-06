# S012500 implementation

Source: `vendor/linux/include/asm-generic/audit_dir_write.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The leased task maps that reincludable C preprocessor initializer fragment to
`src/include/asm-generic/audit_dir_write.rs`. The frozen header closure names
the x86_64 IA32 and AArch64 native consumers. Its inclusion from
`asm-generic/audit_write.h` also produces the x86_64 native and AArch64
AArch32-compat instances used by the selected audit translation units.

The source contains no declarations or storage. Four exported,
caller-supplied macros preserve the exact conditional membership and order for
those frozen contexts, with explicit `u32` values matching the consumers'
`unsigned int` elements. Each expansion deliberately leaves the `~0U`
sentinel to the consuming array, matching the C inclusion sites.

No compiler, formatter, build, test, linker, emulator, debugger, or benchmark
was run.
