# Applier resolution — S012510

Applier: `gpt-5.6-terra` (high)

I reopened the complete pinned
`vendor/linux/include/asm-generic/bitops/builtin-ffs.h` at revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`, its AArch64 selector
`arch/arm64/include/asm/bitops.h`, the generic `int` fallback in
`include/asm-generic/bitops/ffs.h`, the frozen AArch64 configuration, the
header-closure record, the candidate, and both reviews.  No compiler,
formatter, analyzer, build, test, or runtime tool was used.

## Finding dispositions

### P1 — accepted; candidate is insufficient and cannot be corrected within this task

The operative upstream contract is the function-like macro
`#define ffs(x) __builtin_ffs(x)`.  Its operand is converted at the C builtin
boundary to `int`; the result is `int`, with zero mapping to zero and a
nonzero converted 32-bit value mapping to the one-based least-significant set
bit position.

The frozen selected Rust-translation callers are not all `int` inputs.  They
include `u32`/`unsigned int` values (for example the AArch64 VGIC
`irq->source`, `kernel/softirq.c`'s `pending`, and
`net/ethtool/tsconfig.c`'s request masks) and AArch64 `unsigned long` values
(for example `net/core/gro.c`'s `bitmask` and `mm/readahead.c`'s
`max_pages`).  The AArch64 UAPI establishes `__u32` as `unsigned int` and
`__s32` as signed `int`; its selected kernel configuration is 64-bit, and the
architecture headers establish a 64-bit `long`.  The candidate's
`ffs(i32) -> i32` consequently rejects selected operands before the required
conversion.

Replacing it with `as i32` is not accepted: it would choose a Rust
truncating conversion for every out-of-range unsigned value, while the source
contract reaches the implementation-defined C conversion to signed `int`.
The frozen source, configuration, and recorded metadata prove the relevant
widths but do not record the frozen compiler's result for that conversion.
No compiler-derived evidence may be obtained during Phase 1.

A local Rust macro would require downstream `ffs!(expr)` syntax, rather than
the mapped function-call surface.  A generic function would require a shared,
audited input-conversion trait and imports/namespace policy across all selected
callers.  Neither exists in the frozen task guidance; module indexes are
expressly deferred until all file tasks finish.  Adding either would be new
shared machinery, and limiting it to selected caller types would be a
convenience restriction rather than the source macro contract.

Disposition: **BLOCK** S012510.  Do not mark `DONE`; retain the candidate as
evidence only and do not add caller-specific casts.

### R1 — accepted; same blocker as P1

The Rust review correctly identifies that Rust has no implicit C-style
conversion at a function call.  The candidate cannot represent the selected
unsigned operands, and no source-proven replacement conversion boundary is
available within this isolated header task.  Disposition: **BLOCK**, as above.

## Pending-record resolution

- `_ASM_GENERIC_BITOPS_BUILTIN_FFS_H_`, `ifndef@2`, and `endif@15` are C
  preprocessor include-once controls.  They have no runtime, storage, ABI,
  ownership, locking, or cleanup behavior; any Rust module inclusion policy is
  shared module-index work and is not implemented by this task.
- `ffs` is the sole operative behavior.  Its zero-result and one-based
  bit-position semantics are understood, but its selected C conversion and
  macro-interface contract cannot be faithfully mapped under the frozen
  source-only evidence and isolated-file boundary.  This record remains the
  explicit blocker rather than being closed by an unverified conversion.

## Blocker statement for the queue

`include/asm-generic/bitops/builtin-ffs.h` is a C macro whose selected callers
depend on implicit conversion to `int`, including 32-bit unsigned and 64-bit
unsigned-long operands.  Faithful Rust mapping requires an audited shared
conversion/macro interface and exact frozen-compiler out-of-range conversion
evidence; neither is available to this isolated Phase-1 task.  A typed `i32`
function or caller casts changes the contract.
