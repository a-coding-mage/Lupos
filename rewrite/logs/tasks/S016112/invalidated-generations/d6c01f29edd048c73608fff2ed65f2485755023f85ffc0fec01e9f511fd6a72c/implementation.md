# S016112 implementation

- Source: `vendor/linux/include/uapi/linux/elf-em.h` at `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Destination: `src/include/uapi/linux/elf-em.rs`.
- Scope: common (the frozen x86_64 and aarch64 configuration union); no configuration conditional changes are present in the source.
- Translation: all 49 selected ELF machine constant macros are represented in source order as public `i32` constants. Every original unsuffixed C integer literal fits `int`; the Rust type preserves that literal expression width/sign category. The duplicate machine-number alias (`EM_MIPS_RS3_LE` and `EM_MIPS_RS4_BE`) is retained.
- Context checked: UAPI `linux/elf.h` supplies the `Elf32_Half`/`Elf64_Half` machine field; the x86 and arm64 ELF headers consume `EM_X86_64` and `EM_AARCH64`. No storage, ownership, ABI-layout, driver ABI, locking, or lifetime record is defined by this macro-only header.
- No build, formatting, compiler, linker, or test command was run.
