# Parity review — S016005

Reviewed `vendor/linux/include/uapi/asm-generic/hugetlb_encode.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/uapi/asm-generic/hugetlb_encode.rs`, source-only, for the frozen
common x86_64/aarch64 scope.

## Result

**REJECT: two corrections are required before this task can be accepted.**

### P1 — UAPI SPDX identifier was changed

Linux line 1 is `/* SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note */`.
Candidate line 1 instead declares `// SPDX-License-Identifier: GPL-2.0-only`.
This is a material, unallowlisted change to the upstream UAPI licensing
identifier. `rewrite/BRANDING_ALLOWLIST.tsv` contains no permitted rename or
license delta. Replace the candidate SPDX identifier with the exact upstream
`GPL-2.0 WITH Linux-syscall-note` expression while retaining the required
provenance lines.

### P2 — the two unsuffixed base macros have the wrong signedness

Linux lines 20 and 21 define `HUGETLB_FLAG_ENCODE_SHIFT` as decimal `26` and
`HUGETLB_FLAG_ENCODE_MASK` as hexadecimal `0x3f`, respectively. Both literals
fit in `int`; therefore each macro has signed 32-bit `int` type on both frozen
targets. Candidate lines 9-10 declare both as `u32`, changing their direct
constant type and the type exported to aliases such as `MAP_HUGE_SHIFT` /
`MAP_HUGE_MASK`, `MFD_HUGE_SHIFT` / `MFD_HUGE_MASK`, and `SHM_HUGE_SHIFT` /
`SHM_HUGE_MASK` (see `include/uapi/linux/{mman,memfd,shm}.h`). Translate these
two definitions as `i32` (or an exact frozen-target C `int` alias), not `u32`.

## Exhaustive comparison notes

- The scope row is `RUST_TRANSLATE`, architecture `common`; header-closure
  evidence selects it for both configurations (aarch64: 49 consumers;
  x86_64: 43 consumers). The source contains no configuration-selected branch
  apart from the C include guard.
- Candidate provenance names the correct Linux path, task ID, common
  architecture set, and pinned SHA. There is no allowlisted branding delta.
- The C include guard (`_ASM_GENERIC_HUGETLB_ENCODE_H_`, source lines 1-2 and
  37) only controls repeated textual C inclusion. A Rust module is loaded once
  rather than textually included; it has no Rust UAPI/linkage equivalent and
  needs no exported replacement.
- `HUGETLB_FLAG_ENCODE_16KB`, `64KB`, `512KB`, `1MB`, `2MB`, `8MB`, `16MB`,
  `32MB`, `256MB`, `512MB`, `1GB`, `2GB`, and `16GB` (Linux lines 23-35) are
  all present under the original names. Each uses `u32`, matching the `U`
  suffixed unsigned-int left operand in C, and preserves the source exponent
  and the shift by 26. This includes the 16GB encoding (`34U << 26`), whose
  `u32` result is required.
- The header defines no functions, objects, layouts, calling conventions, or
  linker-visible symbols. `pub const` consequently introduces no missing C
  linkage requirement. No ownership, lifetime, locking, RCU, allocation, or
  cleanup behavior exists in this constants-only header.
- No compiler, formatter, linker, test, or runtime tool was invoked.

## Required applier disposition

Correct P1 and P2, then recheck all thirteen encoded constants remain `u32`
with the exact source exponents and that the two base macros are signed C-int
constants. No other source change is requested by this review.
