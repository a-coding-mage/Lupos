# Parity review — S012510

Reviewer: parity reviewer (`gpt-5.6-terra`, high)

Scope reviewed: `src/include/asm-generic/bitops/builtin-ffs.rs` against the
complete pinned `include/asm-generic/bitops/builtin-ffs.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, with the frozen AArch64 selection
and its bitops include/call context. No compiler, formatter, analyzer, build,
test, or runtime tool was used.

## Finding P1 — `ffs` no longer has the selected macro's `int`-conversion call interface

The upstream operative definition is exactly:

```c
#define ffs(x) __builtin_ffs(x)
```

`__builtin_ffs` has `int` input and `int` output. As a C macro it accepts a
single scalar expression, evaluates it once through the builtin, and applies
the target's conversion to 32-bit `int` before finding the first bit. The
candidate instead exposes only `pub const fn ffs(x: i32) -> i32`. That rejects
the selected unsigned call expressions rather than preserving the macro's
conversion behavior. In the frozen AArch64 source, for example,
`arch/arm64/kvm/vgic/vgic-v2.c:271` and `vgic-v3.c:352,385` pass
`irq->source` to `ffs`; those receive the result into `u32`. Likewise,
`arch/arm64/lib/insn.c:1399` passes `esz`, declared `unsigned int` at line
1349. The C macro/builtin converts each to `int` (AArch64's selected `int` is
32 bits); the candidate has no corresponding accepted input or conversion
boundary.

This is not limited to ergonomics: values such as `0x80000000u32` must be
interpreted after the C `int` conversion and still produce 32, while values
outside a direct Rust `i32` call cannot be expressed through this API without
call-site redesign. The candidate's `i32` body does correctly return zero for
zero and otherwise gives 1..=32 for an already-converted 32-bit value, but it
does not implement the selected macro's interface or C promotion/conversion
semantics.

Required resolution: preserve the `__builtin_ffs`/32-bit-`int` conversion
contract at the Rust boundary for all selected operand forms, without changing
the source's one-evaluation behavior.

## Verified parity points

- The Rust provenance records the exact Linux path, pinned revision, AArch64
  scope, and task ID; its SPDX identifier matches the upstream file.
- `include/asm-generic/bitops/builtin-ffs.h` is selected for AArch64
  (`S012510`, 8,827 header-closure consumers) and is directly included by
  `arch/arm64/include/asm/bitops.h` after `<linux/bitops.h>` establishes the
  required include context.
- For a value already represented as `i32`, the candidate's zero case and
  least-significant-set-bit numbering match `__builtin_ffs`, including a set
  sign bit yielding 32.

## Result

Rejected pending resolution of P1. No source, queue, or evidence file other
than this assigned parity report was modified.
