# S000188 implementation

BLOCKED before source creation. The pinned Linux header
`arch/arm64/include/asm/unistd.h` is a preprocessor control header, not a
self-contained declaration: it conditionally defines the `__ARCH_WANT_*` and
`__ARM_NR_*` inputs consumed by the generated `asm/unistd_64.h` include, then
defines `NR_syscalls` as `__NR_syscalls`. The referenced `unistd_64.h` is a
generated/configuration-dependent syscall-number and syscall-selection
interface; its generated body is not present as a source file in the pinned
tree. The frozen symbol records retain the macro and CONFIG_COMPAT branches as
`PENDING_REVIEW`, and no Rust module or build bridge is specified that can
preserve C preprocessor include/macro expansion and the generated UAPI ABI.

Creating a Rust file containing constants or guessed `cfg` branches would
change the syscall namespace and configuration semantics. Exact parity cannot
be established from the frozen source and manifests, so no destination source
or candidate snapshot was written. No compiler, formatter, linker, test,
runtime, or historical Rust source was used.
