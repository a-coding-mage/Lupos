# Parity review — S012494 (attempt 1, P02)

Result: **FINDINGS**.  Source-only review; no compiler, formatter, runtime, or
historical Lupos source was used.

## P1 — `ACPI_PROC_CAP_*` macro expression/type contract was changed without an ABI record

The complete pinned header defines every leaf mask as an unsuffixed hexadecimal
integer literal and each composite as an expression made from those macros:
`vendor/linux/include/acpi/proc_cap_intel.h:11-38`.  Thus the replacement-token
expressions have C `int` type (and retain C usual arithmetic conversion and
bitwise-complement behavior), rather than being an ABI-declared `u32` object.
The candidate turns all fifteen public expansion sites into `pub const ...:
u32` (`src/include/acpi/proc_cap_intel.rs:9-40`).

This is observable in the selected callers.  `arch_acpi_set_proc_cap_bits()`
uses composites and leaves in `|=` and uses `~(ACPI_PROC_CAP_C_C1_FFH |
ACPI_PROC_CAP_C_C2C3_FFH)` at
`vendor/linux/arch/x86/include/asm/acpi.h:117-142`; the source expression is
formed as `int` before assignment to `u32`.  The Xen path forms composite masks
and consumes individual leaves at `vendor/linux/arch/x86/xen/enlighten_pv.c:347-358`.
The task's `rewrite/ABI.tsv` contains no record for this header, so there is no
frozen source-derived ABI/type decision that establishes an intentional and
exact conversion to `u32`, nor one that establishes how all translation-unit
contexts (especially complement and integer promotion) remain equivalent.

Affected Linux symbols: `ACPI_PROC_CAP_P_FFH`,
`ACPI_PROC_CAP_C_C1_HALT`, `ACPI_PROC_CAP_T_FFH`,
`ACPI_PROC_CAP_SMP_C1PT`, `ACPI_PROC_CAP_SMP_C2C3`,
`ACPI_PROC_CAP_SMP_P_SWCOORD`, `ACPI_PROC_CAP_SMP_C_SWCOORD`,
`ACPI_PROC_CAP_SMP_T_SWCOORD`, `ACPI_PROC_CAP_C_C1_FFH`,
`ACPI_PROC_CAP_C_C2C3_FFH`, `ACPI_PROC_CAP_SMP_P_HWCOORD`,
`ACPI_PROC_CAP_COLLAB_PROC_PERF`, `ACPI_PROC_CAP_EST_CAPABILITY_SMP`,
`ACPI_PROC_CAP_EST_CAPABILITY_SWSMP`, and
`ACPI_PROC_CAP_C_CAPABILITY_SMP`.

The applier must establish the exact cross-task C-expression / Rust call-site
mapping and signedness/promotion rule from frozen local evidence, or block the
task.  Merely asserting `u32` constants is not a source-proven equivalent.

## P2 — include-guard macro behavior and candidate snapshot are not preserved/auditable

The header's selected conditional and operative macro are the actual
preprocessor state transition `#ifndef __PROC_CAP_INTEL_H__` / `#define
__PROC_CAP_INTEL_H__` / `#endif` at
`vendor/linux/include/acpi/proc_cap_intel.h:8-40`.  The candidate contains no
mapping for a C-visible include guard and only states in prose that a Rust
module boundary represents it (`src/include/acpi/proc_cap_intel.rs:1-5` and
`rewrite/logs/tasks/S012494/candidate.diff`).  No frozen manifest or ABI record
establishes that this source-level preprocessor contract can be discarded for
all selected C consumers.

Additionally, `candidate.diff` is only a wildcard summary (`#define
ACPI_PROC_CAP_* ...` / `pub const ACPI_PROC_CAP_*: u32 = ...`) rather than a
line-resolving candidate snapshot.  It therefore cannot bind each selected
macro/conditional in `rewrite/SYMBOLS.tsv` to the reviewed Rust source or
support an exhaustive later resolution.  That is especially material here
because every selected record is still marked `PENDING_REVIEW` before the
candidate's blanket `COMPLETE` proposal.

Affected Linux symbol: `__PROC_CAP_INTEL_H__`, plus every capability macro
listed in P1.

## Scope checked

Complete pinned header read: `vendor/linux/include/acpi/proc_cap_intel.h`.
Direct selected consumer contexts checked: `vendor/linux/arch/x86/include/asm/acpi.h:117-142`,
`vendor/linux/arch/x86/xen/enlighten_pv.c:347-358`,
`vendor/linux/drivers/acpi/acpi_processor.c:572-590`, and
`vendor/linux/drivers/acpi/processor_pdc.c:15-27`.
