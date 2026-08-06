# S012499 implementation

Source: `vendor/linux/include/asm-generic/audit_change_attr.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The P02 lease, required branch, queue fingerprint, Phase 0 source revision,
and both selected architecture records were verified before editing. This
header is an untyped C initializer fragment, included by four selected source
contexts: x86_64 native (`arch/x86/kernel/audit_64.c`), x86_64 IA32
(`arch/x86/ia32/audit.c`), AArch64 native (`lib/audit.c`), and AArch64
AArch32-compat (`lib/compat_audit.c`). Each receiving C array owns the trailing
`~0U` sentinel and its `unsigned int` storage.

The Rust destination provides one exported consumer macro for each selected
preprocessor context. Each invokes its consumer once with the upstream entries
in source order, with every `#ifdef __NR_*` membership resolved from the pinned
architecture syscall tables. Entries are explicit `u32`, matching the C array
element type. No storage, sentinel, ABI object, or replacement collection is
introduced here; those remain the responsibility of the translated receiving
source just as in C.

Value evidence is the pinned `arch/x86/entry/syscalls/syscall_64.tbl`,
`arch/x86/entry/syscalls/syscall_32.tbl`,
`arch/arm64/tools/syscall_32.tbl`, and `include/uapi/asm-generic/unistd.h`.
The AArch64 native context selects the asm-generic syscall definitions; its
compat context selects the AArch32 table. No compiler, formatter, build, test,
linker, or runtime command was run.
