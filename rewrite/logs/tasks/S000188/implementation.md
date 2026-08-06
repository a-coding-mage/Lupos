# Implementation — S000188

Source oracle: `vendor/linux/arch/arm64/include/asm/unistd.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The frozen AArch64 configuration has `CONFIG_COMPAT=y`, so the candidate
retains all twelve compatibility selection markers and the four ARM-private
SVC-number expressions. It also retains the unconditional clone and new-stat
selection markers.

`asm/unistd_64.h` is the generated build-metadata row `S012326`, not a Rust
translation task. Its frozen generated `__NR_syscalls` value is 472; the
header's `NR_syscalls` alias is represented as the Rust compile-time constant
`NR_syscalls` with that value. The generated header remains owned by the later
build-metadata workflow.

No compiler, formatter, linker, test, or runtime command was run.
