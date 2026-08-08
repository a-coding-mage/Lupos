# S016215 implementation

- Pipeline: P01, attempt 1; implementer: Luna medium.
- Linux source: `vendor/linux/include/uapi/linux/kernel-page-flags.h` at revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Destination: `src/include/uapi/linux/kernel-page-flags.rs`.
- Frozen architecture membership is `common`; the scope and symbol manifests select the same header for x86_64 and AArch64 through the common UAPI path.
- The complete pinned header contains only its SPDX notice, include guard, comments, and 27 object-like integer macros. The include guard has no Rust runtime analogue and does not expose an API symbol.
- Each KPF macro is represented as a public `i32` constant, preserving the Linux UAPI `int` value and exact numeric sequence: KPF_LOCKED through KPF_BUDDY are 0..10, KPF_MMAP through KPF_NOPAGE are 11..20, and KPF_KSM through KPF_PGTABLE are 21..26.
- `KPF_ERROR` retains the source's “Now unused” comment, and the source grouping comments are retained.
- No functions, structs, unions, configuration-dependent branches, unsafe operations, allocations, synchronization, or error paths occur in this header. No dependencies beyond the header itself were required.
- No compiler, formatter, linker, test, runtime, or Git mutation was used.
