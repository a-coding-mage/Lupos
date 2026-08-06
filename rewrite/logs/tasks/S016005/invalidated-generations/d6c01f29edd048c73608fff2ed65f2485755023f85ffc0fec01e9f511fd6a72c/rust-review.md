# Rust semantics review — S016005

## Result

Accepted: no Rust-semantics findings. This was a source-only review; no
compiler, formatter, test, linker, or rust-analyzer diagnostic was invoked.

## Sources and scope reviewed

- Pinned source: `vendor/linux/include/uapi/asm-generic/hugetlb_encode.h`,
  lines 1–37, at revision
  `425f94c2954b1fe80ebdbf9b29854e89750355df` (`vendor/linux.SHA`).
- Candidate: `src/include/uapi/asm-generic/hugetlb_encode.rs`, lines 1–24.
- Phase-0 scope: `rewrite/SCOPE.tsv:16006` classifies this common UAPI header
  as `RUST_TRANSLATE`, with header-closure evidence for both frozen targets
  (49 AArch64 and 43 x86_64 consumers).
- Symbol inventory: `rewrite/SYMBOLS.tsv:320481–320516` records the include
  guard and all 15 operative macros for both frozen architectures.

## Integer and shift analysis

The C shift-count and mask literals at source lines 20–21 are unsuffixed
integer constants. The thirteen encoded-size macros at lines 23–35 have an
`unsigned int` left operand (`N U`) and a shift count of 26. On both selected
Linux targets, that operand is 32 bits; each shift is within range and produces
an unsigned 32-bit flag value. In particular, the largest result,
`34U << 26`, is `0x8800_0000`, which remains defined unsigned arithmetic.

The candidate makes the flag representation explicit as `u32`: lines 12–24
preserve every source operand `N`, retain the 26-bit shift, and therefore yield
the same encoded 32-bit bit patterns. Its `u32` shift count at line 9 is valid
for these expressions because 26 is less than the 32-bit left operand width;
there is no signed left-shift, overflow, truncation, or debug/release-dependent
operation. The `u32` mask at line 10 likewise has the same `0x3f` bits and is
the appropriate explicit representation when combined with the unsigned
encoded flag constants. The source macros emit no object or FFI symbol, so the
explicit Rust constant types do not change a layout, linkage, or calling
convention ABI.

The candidate exports each non-guard macro name exactly once. The C include
guard at source lines 1–2 is correctly not made into a public Rust value: Rust
module loading supplies its corresponding once-only inclusion property.

## Other Rust-semantics checks

- Provenance lines 1–5 match the task, source path, common architecture scope,
  and pinned revision.
- This header has no functions, storage, layouts, pointers, ownership,
  concurrency, FFI boundary, or `unsafe` operation to introduce Rust lifetime,
  aliasing, `Send`/`Sync`, drop-order, or panic behavior.
- The candidate introduces no test configuration, placeholder, mutable state,
  unauthorized branding, or extra public operative item.

No change is requested from the applier on Rust-semantics grounds.
