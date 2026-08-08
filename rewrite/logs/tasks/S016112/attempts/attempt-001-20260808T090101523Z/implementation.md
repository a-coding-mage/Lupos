Task S016112 implementation record

- Lease: P01, owner codex-root-p01, attempt 1; branch verified as feat/bun-like-rewrite-test.
- Source: vendor/linux/include/uapi/linux/elf-em.h at Linux revision 425f94c2954b1fe80ebdbf9b29854e89750355df.
- Destination: src/include/uapi/linux/elf-em.rs.
- Scope: common source class; frozen architecture consumers are x86_64 and AArch64. The source is a guarded UAPI constant header with no includes, types, callers, callees, Kconfig branches, or Kbuild code requiring additional translation context.
- Translation: preserved all 49 EM_* definitions and exact numeric values, including duplicate EM_MIPS_RS3_LE/EM_MIPS_RS4_BE value 10 and hexadecimal legacy values. C preprocessor guard has no Rust item because module single-definition behavior supplies the boundary.
- Representation: constants use `i32`, matching the existing UAPI integer-constant convention and the C unsuffixed integer expressions (all values fit exactly).
- No unsafe code, tests, stubs, compiler/formatter/runtime commands, or other task files were used.
