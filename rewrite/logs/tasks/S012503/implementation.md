# S012503 implementation

Fresh attempt 2 for `include/asm-generic/audit_write.h`, leased by P01 as
`codex-root-cont3-20260806-p01`.

The source is a textual initializer fragment, not a declaration. It first
includes `audit_dir_write.h` and then emits the audit write syscall numbers
selected by the active syscall-number definitions. The frozen inclusion
contexts are x86_64 native, x86_64 IA32, AArch64 native, and AArch64
AArch32-compat. This translation exports one consumer macro per context so a
consumer supplies its own `unsigned int`-compatible static initializer and
the following `~0U` sentinel, matching the C sites.

The AArch64 native expansion deliberately retains duplicate `truncate` /
`truncate64` and `ftruncate` / `ftruncate64` values because both spellings are
defined by the generic syscall header and the source emits each conditional
entry.

Pinned oracle: `vendor/linux/include/asm-generic/audit_write.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.
