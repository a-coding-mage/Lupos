# Rust review — S000496

Reviewed `src/arch/x86/include/asm/cpufeatures.rs` independently against
`vendor/linux/arch/x86/include/asm/cpufeatures.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, limited to the frozen x86_64
configuration.

## Result

No Rust-specific blocking findings.

- The candidate exports the complete x86_64-selected macro set: all 471
  upstream capability/bug definitions except `X86_BUG_ESPFIX`, which is
  correctly absent because upstream guards it with `CONFIG_X86_32` at lines
  535–541.
- `NCAPINTS`, `NBUGINTS`, each `X86_FEATURE_*` index, and the two-word
  `X86_BUG` index calculation retain their upstream word-times-32-plus-bit
  values. The chosen `u32` domain represents every selected index (0–692) and
  agrees with downstream capability APIs whose `feature` parameters are
  `unsigned int` (for example `clear_cpu_cap()` in
  `arch/x86/kernel/cpu/cpuid-deps.c:150`). No layout, linkage, FFI, ownership,
  unsafe, or drop behavior is introduced by this constants-only header.
- The provenance names the pinned source, exact revision, x86_64-only scope,
  and task ID. No unauthorized architecture branch, configuration emulation,
  or executable/unsafe surface is present.

No build, formatter, test, or compiler command was run.
