# Parity review — S012494 (slot 1)

## Verdict

Accepted: no parity findings.

## Evidence reviewed

- Pinned source: `vendor/linux/include/acpi/proc_cap_intel.h` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df` (lines 1–40).
- Candidate: `src/include/acpi/proc_cap_intel.rs` (lines 1–36).
- Frozen scope/queue records: `S012494`, x86_64, header-closure consumer of
  `arch/x86/kernel/acpi/boot.o`; Phase 0 identity pins the same revision and
  x86_64 configuration.
- Relevant pinned consumers: `arch/x86/include/asm/acpi.h:117–152` and
  `arch/x86/xen/enlighten_pv.c:350–357`.

## Exhaustive comparison

The source has no functions, types, statics, includes, configuration-selected
branches, or externally linked objects.  Its include guard has no Rust runtime,
layout, or linkage counterpart.  All fifteen operative macros are present with
their original names: the twelve primitive masks and the three composite masks
`ACPI_PROC_CAP_EST_CAPABILITY_SMP`,
`ACPI_PROC_CAP_EST_CAPABILITY_SWSMP`, and
`ACPI_PROC_CAP_C_CAPABILITY_SMP`.

Each primitive mask retains its source value.  Each composite retains the same
operand set and bitwise-OR result (respectively `0x000b`, `0x082b`, and
`0x031a`).  The candidate's explicit `u32` representation matches every
operative pinned use: `arch_acpi_set_proc_cap_bits` mutates a `u32` capability
word, and the Xen PDC buffer is a `u32` word array.  In particular, the source
complement-and-AND path for the two FFH masks has the same 32-bit bit pattern
with these constants.

The required provenance names the exact source path, pinned revision, x86_64
architecture, and task ID.  No branding delta, omitted source mechanism,
additional behavior, placeholder, test, or driver translation is present.

No build, formatter, test, runtime, or compiler command was run for this
review.
