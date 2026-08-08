# Applier resolution — S016011, attempt 1

Scope reopened: pinned `vendor/linux/include/uapi/asm-generic/mman-common.h`
(complete file), its direct `asm-generic/mman.h` and x86_64/AArch64 UAPI
inclusion paths, the direct `include/linux/mman.h` consumer context, the
candidate and candidate diff, both reviews, and the task-local semantic
closure records.  This was source inspection only; no compiler, formatter,
linker, test, runtime command, or historical Rust source was used.

## Finding dispositions

### PARITY-001 — accepted; candidate correction required

The pinned definitions at `mman-common.h:10-91` are unsuffixed integer
literals.  The largest is `MAP_UNINITIALIZED` at line 33 (`0x4000000`), which
fits the selected targets' signed 32-bit `int` domain; the pinned UAPI integer
headers define `__s32` as signed `int` and `__u32` as unsigned `int`
(`include/uapi/asm-generic/int-ll64.h:26-27` and `int-l64.h:26-27`).  The
operand macros in `PKEY_ACCESS_MASK` at lines 89-92 are therefore `int`, and
the `|` expression is also `int`.

The candidate instead fixes every mapped macro to `u32`.  That changes the
macro-expression domain and cannot be accepted.  The replacement candidate
must declare every value macro from lines 10-91 as `i32`, retaining
`PKEY_ACCESS_MASK` as the computed `i32` bitwise OR of its `i32` operands.
Any width conversion required by a later translated consumer belongs at that
consumer, as illustrated by `arch_validate_prot(unsigned long prot, ...)` in
`include/linux/mman.h:99-110`.

### RUST-S016011-001 — accepted; same correction as PARITY-001

This independently identifies the same material defect.  `i32` preserves the
signed, in-range C `int` expression domain for all of these object-like
macros; `u32` does not preserve signed complement, comparison, or mixed
arithmetic behaviour.  No separate ownership, unsafe, layout, or lifetime
change is required for this constant-only header.

### PARITY-002 — accepted as a semantic-closure/attestation defect; no Rust
value symbol should be added

The C guard is present at `mman-common.h:2-3,94`; `asm-generic/mman.h:4`
includes this header, and both selected architecture UAPI headers include that
generic header.  A pinned-tree search finds `__ASM_GENERIC_MMAN_COMMON_H` only
as this header's guard (apart from the separate tools header copy), so no
selected source consumes it as a value or feature-test contract.

Its exact C role is to emit the header body only once per preprocessing
translation unit.  The Rust counterpart is one declaration/loading of the
path-preserving module by the deterministic module-index phase.  Introducing a
Rust `__ASM_GENERIC_MMAN_COMMON_H` constant would be incorrect: the C macro
has an empty replacement list and is not a Rust integer value or exported ABI
symbol.  The semantic closure must instead explicitly map the two `ifndef` /
`endif` records and the guard record for each selected architecture to this
module-once invariant.

The existing proposal's bare `SOURCE_REVIEWED_VALUE` entries do not provide
that mapping, and the Rust review attestation has an empty `record_keys` field.
It cannot close the guard or conditional records and must not be used for an
apply or DONE decision.

## Required controlled follow-up

This task is **not BLOCKED**: the pinned source establishes both the `i32`
correction and the non-value guard mapping.  It is also not eligible for
`APPLYING`, because the candidate must change and the Rust semantic attestation
is incomplete.  The coordinator should perform a controlled requeue to
`IMPLEMENTED` (P01, attempt 1) with a reason covering the `u32` to `i32`
correction and incomplete guard-record attestation.  The implementer must
produce the corrected candidate and refreshed candidate evidence; then the
semantic-closure proposal must give non-empty record-key coverage for all guard
and conditional records.  Both independent reviews must be rerun and resealed
against that new candidate/proposal before any later `APPLYING` transition.

No queue transition or source change was made by this applier pass.
