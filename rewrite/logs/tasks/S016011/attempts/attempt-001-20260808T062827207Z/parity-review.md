# Parity review — S016011 (slot 1)

Reviewed the pinned `vendor/linux/include/uapi/asm-generic/mman-common.h`, the
current candidate `src/include/uapi/asm-generic/mman-common.rs`, task-local
candidate diff, and frozen task records for both x86_64 and aarch64. No build,
formatter, test, compiler, or diagnostics were invoked.

## Findings

### PARITY-001 — all value macros have the wrong C expression type

Linux symbols: `PROT_READ`, `PROT_WRITE`, `PROT_EXEC`, `PROT_SEM`,
`PROT_NONE`, `PROT_GROWSDOWN`, `PROT_GROWSUP`, `MAP_TYPE`, `MAP_FIXED`,
`MAP_ANONYMOUS`, `MAP_POPULATE`, `MAP_NONBLOCK`, `MAP_STACK`, `MAP_HUGETLB`,
`MAP_SYNC`, `MAP_FIXED_NOREPLACE`, `MAP_UNINITIALIZED`, `MLOCK_ONFAULT`,
`MS_ASYNC`, `MS_INVALIDATE`, `MS_SYNC`, every `MADV_*` value macro,
`MAP_FILE`, `PKEY_UNRESTRICTED`, `PKEY_DISABLE_ACCESS`,
`PKEY_DISABLE_WRITE`, and `PKEY_ACCESS_MASK`.

Local evidence: the pinned Linux definitions at lines 10–91 use unsuffixed
integer literals; their greatest literal, `MAP_UNINITIALIZED` (`0x4000000` at
line 33), is representable as `int` on both selected targets. Consequently
each literal macro has C type `int`; `PKEY_ACCESS_MASK` at lines 91–92 applies
`|` to two `int` operands and likewise has type `int`. The candidate instead
declares every value macro as `pub const ...: u32` (lines 7–76), including the
two operands and result of `PKEY_ACCESS_MASK`. This changes integer promotion,
signed comparison, conversion, and bitwise-expression behavior at every
consumer rather than preserving the header’s macro-expression contract. The
pinned `include/linux/mman.h` confirms that widening is performed by its
individual use sites (for example `arch_validate_prot(unsigned long prot, ...)
at lines 73–75), not by the generic UAPI definitions themselves.

Required resolution: retain the C `int` expression domain for these symbols
(and preserve the computed `PKEY_ACCESS_MASK` in that domain), unless a
pinned per-symbol ABI record establishes a different required Rust-facing
contract. None was present in the task’s frozen ABI rows.

### PARITY-002 — selected include-guard symbol and conditional branch are absent

Linux symbol: `__ASM_GENERIC_MMAN_COMMON_H`.

Local evidence: the pinned header implements its one-definition/include
mechanism with `#ifndef __ASM_GENERIC_MMAN_COMMON_H` at line 2, `#define
__ASM_GENERIC_MMAN_COMMON_H` at line 3, and the matching `#endif` at line 94.
`SYMBOLS.tsv` records the guard as an `operative_macro` and records both
conditionals for each selected architecture. The candidate has neither a
mapping for the guard symbol nor an equivalent explicitly documented mechanism;
it begins directly with Rust constants (line 7). The direct pinned consumers
`arch/x86/include/uapi/asm/mman.h` and
`arch/arm64/include/uapi/asm/mman.h` include the generic mman header, which in
turn includes this common header (`include/uapi/asm-generic/mman.h:5`). A Rust
module’s loading rules may address duplicate module definitions, but they do
not provide the selected preprocessor symbol or its conditional visibility.

Required resolution: provide and source-justify the Rust mapping for the
selected guard and both selected conditional records, or mark the task blocked
if the exact observable contract cannot be established.

## Checked without an additional finding

The candidate contains every non-guard value macro named by the pinned header;
the numerical spellings evaluate to the same values, and `PKEY_ACCESS_MASK`
retains an OR expression. The candidate’s SPDX/provenance identifies the
pinned source and revision, and no branding delta is present in the frozen
allowlist. These observations do not cure the two findings above.

Result: **FINDINGS**. The sealed semantic proposal must not be approved for
this candidate.
