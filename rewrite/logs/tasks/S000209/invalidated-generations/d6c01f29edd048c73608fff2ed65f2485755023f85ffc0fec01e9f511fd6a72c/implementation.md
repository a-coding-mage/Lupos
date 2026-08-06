# S000209 implementation

Task `S000209` translates `arch/arm64/include/uapi/asm/auxvec.h` to
`src/arch/arm64/include/uapi/asm/auxvec.rs` for the frozen aarch64 input.

Pinned source and Phase 0 identity were checked before editing:

- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Queue fingerprint: `af93adda6e7845ec178dc63a9462f88384392f78353165ea5a583ef78fcf423c`.
- The task is an aarch64, dependency-free `RUST_TRANSLATE` header selected by
  `rewrite/metadata/header_closure.tsv`.

The complete oracle has three operative macros.  They are represented as public
64-bit auxiliary-vector key/count constants with their exact values: vDSO key
`33`, minimum signal-stack key `51`, and ARM64 `ARCH_DLINFO` entry count `2`.
ARM64's ELF auxiliary vector is populated through `elf_addr_t`, which resolves
to `Elf64_Off` for the native ARM64 configuration.  No C include guard is
needed for the Rust module.

Relevant pinned consumers and context inspected:

- `include/uapi/linux/auxvec.h` imports this architecture header and leaves its
  generic `AT_MINSIGSTKSZ` fallback inactive when this definition is present.
- `arch/arm64/include/asm/elf.h` emits `AT_SYSINFO_EHDR` and conditionally emits
  `AT_MINSIGSTKSZ` in its two-entry `ARCH_DLINFO` sequence.
- `fs/binfmt_elf.c` writes auxiliary-vector tag/value pairs as `elf_addr_t`;
  `arch/arm64/kernel/signal.c` computes the value supplied for the latter tag.

There are no functions, state, unsafe operations, configuration branches beyond
the C header guard, or allocation/lifetime/locking behavior in this source.
No compiler, formatter, linker, test, runtime, or historical Rust source was
used.
