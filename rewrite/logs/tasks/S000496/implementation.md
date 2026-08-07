# Implementation record — S000496

- Linux source: `vendor/linux/arch/x86/include/asm/cpufeatures.h`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Destination: `src/arch/x86/include/asm/cpufeatures.rs`
- Architecture: `x86_64`
- Queue lease: pipeline `P02`, attempt `1`, owner `codex-root-repair-20260807-p02`.

The fresh translation preserves `NCAPINTS`, `NBUGINTS`, all selected
`X86_FEATURE_*` and `X86_BUG_*` bit positions, comments, and arithmetic.  The
C function-like `X86_BUG(x)` macro is represented by a `const fn` with the
same `NCAPINTS * 32 + x` calculation, so callers retain compile-time arithmetic
and argument evaluation semantics.  The `CONFIG_X86_32` conditional is retained
as a Rust configuration gate around `X86_BUG_ESPFIX`; it is inactive for the
leased x86_64 configuration.  The Linux header guard has no Rust counterpart
because the module boundary supplies one definition site.

Semantic closure: every operative define in the pinned header is mapped to a
typed `u32` constant or the function-like constant helper; no feature word,
bit index, bug index, or conditional branch is omitted.  No ABI-bearing
structure or callable symbol is introduced by this constants-only header.
