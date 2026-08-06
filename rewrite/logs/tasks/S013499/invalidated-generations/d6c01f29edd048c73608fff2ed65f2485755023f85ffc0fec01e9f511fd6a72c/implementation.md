# S013499 implementation

- Source: `vendor/linux/include/linux/bcma/bcma_driver_arm_c9.h` at `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Destination: `src/include/linux/bcma/bcma_driver_arm_c9.rs`.
- Scope: `common`; the frozen header-closure inventory selects this header for one x86_64 consumer and four aarch64 consumers.
- Implemented every operative macro in the selected symbol inventory as a public `u32` constant. The pinned in-tree consumer uses the mask and shift constants with `u32` MMIO register values; no types, layouts, functions, conditionally selected branches, ABI linkage, ownership, or synchronization behavior are defined by this header.
- The C include guard has no Rust runtime or ABI counterpart. No branding delta was made.
- No compiler, formatter, analyzer, build, test, or runtime command was used.
