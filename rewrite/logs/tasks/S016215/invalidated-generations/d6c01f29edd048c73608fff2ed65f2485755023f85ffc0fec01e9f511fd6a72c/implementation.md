# S016215 implementation

- Leased task: `S016215` on pipeline `P01`; destination: `src/include/uapi/linux/kernel-page-flags.rs`.
- Pinned source reviewed in full: `vendor/linux/include/uapi/linux/kernel-page-flags.h` at `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Scope is `common`; the frozen inventory selects every header macro for both x86_64 and AArch64.
- Translated all 27 `KPF_*` object-like macros into public `i32` constants. The `i32` representation preserves the original unsuffixed C `int` literal values and remains usable as the right operand of the consuming page-flag shifts.
- Preserved each value, ordering group, the `KPF_ERROR` unused note, and the source `GPL-2.0 WITH Linux-syscall-note` identifier. The C include guard has no Rust analogue.
- No configuration branch, type layout, ownership, allocation, locking, FFI linkage, or unsafe operation exists in this header.
- No build, formatter, compiler, test, or runtime command was run.
