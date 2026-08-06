# Parity review — S016112

Reviewed `vendor/linux/include/uapi/linux/elf-em.h` at pinned revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/uapi/linux/elf-em.rs`.

## Evidence checked

- Frozen task/scope records identify this as the common, path-preserving
  translation of the UAPI header for both approved architectures.
- The source has 49 `EM_*` object-like macros and the candidate has 49 public
  `EM_*` constants. A source-order name/value comparison is identical,
  including `EM_S390` remaining after `EM_X86_64`, the equal-valued
  `EM_MIPS_RS3_LE`/`EM_MIPS_RS4_BE` pair, and all hexadecimal spellings.
- Every source replacement token is an unsuffixed integer literal that fits
  the C `int` type on the frozen x86_64 and AArch64 targets. The candidate
  represents each as a signed 32-bit `i32` constant, preserving the C literal
  category, width, signedness, and constant-expression value.
- The header has no configuration branches or attributes. Its only
  preprocessor condition is an include guard, which has no Rust data/API
  counterpart in the path-preserving module translation. There are no layouts,
  functions, storage objects, linkage directives, or driver ABI contracts in
  the source.
- Relevant pinned consumers use these values as ELF-machine integer constants,
  including `EM_X86_64` and `EM_AARCH64`; the candidate retains their original
  public spellings and values. No branding substitutions or extra exported
  machine identifiers were introduced.
- The candidate retains the exact SPDX expression and immutable source,
  revision, architecture, and task provenance. Source ordering and the
  substantive historical/interim comments are retained; comment punctuation
  and line wrapping do not alter UAPI behavior.

## Findings

None. The candidate exhaustively preserves the selected `elf-em.h` UAPI
machine-identifier constant set and its constant semantics.
