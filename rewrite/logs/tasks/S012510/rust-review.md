# Rust review — S012510

Reviewer: `rust_reviewer` (`gpt-5.6-terra`, high)

Scope reviewed: `src/include/asm-generic/bitops/builtin-ffs.rs` against pinned
`vendor/linux/include/asm-generic/bitops/builtin-ffs.h`, with frozen AArch64
inclusion and call-site context only. No compiler, formatter, analyzer, build,
or test was run.

## Finding R1 — Rust input type loses the C macro's required conversion boundary

**Severity: must fix.** The upstream operative definition is the macro
`#define ffs(x) __builtin_ffs(x)` (`vendor/linux/include/asm-generic/bitops/builtin-ffs.h:13`).
The compiler builtin takes an `int` argument, so each C call site retains the C
conversion to `int` at the macro expansion boundary. The candidate instead
publishes only `pub const fn ffs(x: i32) -> i32`
(`src/include/asm-generic/bitops/builtin-ffs.rs:12`). Rust provides no implicit
conversion from the unsigned integer operands used by the selected source, so
these ordinary C uses cannot be expressed through this replacement without
call-site changes or an explicit replacement conversion policy:

- `pending` is declared `__u32` immediately before `ffs(pending)` in
  `vendor/linux/kernel/softirq.c:596,610`.
- `irq->source` is passed at
  `vendor/linux/arch/arm64/kvm/vgic/vgic-v3.c:352` and assigned to `u32`.
- `size` is an `unsigned int` parameter feeding `ffs(size)` at
  `vendor/linux/mm/slab_common.c:709`.

This is not limited to accepting a zero/nonzero value: conversion of unsigned
values outside the signed `int` range is part of the original compiler/C ABI
boundary, while the candidate's signature rejects the operand before its
`trailing_zeros` implementation can run. The applied source must preserve the
macro/builtin input conversion semantics (including the selected unsigned call
sites) without relying on ad-hoc caller casts.

## Checked, no finding

- The AArch64 bitops header selects this exact builtin header
  (`vendor/linux/arch/arm64/include/asm/bitops.h:14-17`); no conditional branch
  in the pinned source changes the `ffs` definition.
- For a valid `i32` input, the candidate returns zero for zero and otherwise
  computes a value in `1..=32`; `trailing_zeros() as i32 + 1` therefore cannot
  overflow (`src/include/asm-generic/bitops/builtin-ffs.rs:12-17`). Negative
  `i32` values retain the relevant 32-bit two's-complement bit positions.
- The candidate contains no `unsafe`, raw-pointer, aliasing, provenance,
  lifetime, or layout boundary. Its immutable source/revision/architecture/task
  provenance matches the pinned header and S012510.

## Disposition

Reject pending resolution of R1. No source changes made by this reviewer.
