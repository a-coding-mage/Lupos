# S000805 resolution

Applier review reopened the complete pinned source
`vendor/linux/arch/x86/include/uapi/asm/vmx.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, its frozen x86_64 consumers
`arch/x86/kvm/trace.h` and `tools/perf/util/kvm-stat-arch/kvm-stat-x86.c`, and
both independent review reports.

## Dispositions

| Finding | Disposition | Resolution |
| --- | --- | --- |
| P1 / R3 | Resolved | The source now retains the exact upstream `GPL-2.0 WITH Linux-syscall-note` SPDX expression and the complete Intel/Qumranet/Avi Kivity/Yaniv Kamay notice block from `vmx.h:1-24`. Immutable provenance remains immediately below it. |
| P2 / R1 | Resolved | Replaced both fixed Rust tuple arrays with exported caller-supplied `macro_rules!` expansions. `VMX_EXIT_REASONS!` supplies the ordered 65 upstream entries, and `VMX_EXIT_REASON_FLAGS!` supplies the single upstream entry, to the receiving table-construction macro. The expansion provides x86_64 `c_ulong` values and NUL-terminated `c"...".as_ptr()` pointers, so the translated selected consumers retain their own `unsigned long` / `const char *` C-compatible table type and static storage rather than inheriting a Rust tuple or `&str` layout. |
| P3 / R2 | Resolved | `VMX_EXIT_REASONS_SGX_ENCLAVE_MODE` is now `i32 = 0x0800_0000`, matching the frozen x86_64 C `int` category. `VMX_EXIT_REASONS_FAILED_VMENTRY` remains `u32 = 0x8000_0000`, because that unsuffixed hexadecimal literal does not fit a signed 32-bit C `int`. |

## Recheck

- All 72 direct value macros retain their original names and values.
- The reason expansion has all 65 upstream pairs in order. `EXIT_REASON_OTHER_SMI`
  and `EXIT_REASON_SEAMCALL` remain direct constants and are intentionally absent
  from that expansion, exactly as in the pinned source.
- The one flag expansion and all three abort-code constants are retained.
- This header has no selected functions, layouts, mutable storage, locking,
  ownership transfer, or unsafe boundary. Its only consumer-dependent ABI is
  the initializer-fragment table materialization resolved above.

No compiler, formatter, build, test, linker, runtime, or benchmark command was
run. No unrelated file was edited.
