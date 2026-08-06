# S014173 implementation

- Leased task: `S014173` on pipeline `P01`; destination: `src/include/linux/kernel-page-flags.rs`.
- Pinned source reviewed in full: `vendor/linux/include/linux/kernel-page-flags.h` at `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Dependency `S016215` (`include/uapi/linux/kernel-page-flags.h`) is `DONE`; its completed destination supplies the UAPI bit positions included by the C header.
- Scope is `common`; frozen x86_64 and AArch64 metadata select this header through built-in `fs/proc/page.o`. All ten selected kernel-only object-like `KPF_*` macros are translated as public `i32` constants, preserving their exact unsuffixed C `int` literal values and the deliberate gap at bit 39.
- The C UAPI inclusion is represented by a public re-export so consumers of this kernel header also receive the completed UAPI `KPF_*` names, matching C preprocessor inclusion.
- The header has no configuration branch, layout, ownership, allocation, locking, FFI linkage, unsafe operation, or runtime behavior. The C include guard has no Rust analogue.
- No build, formatter, compiler, test, rust-analyzer, or runtime command was run.
