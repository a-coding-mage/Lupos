# Resolution — S012494

## Disposition: BLOCKED

The applier reopened the complete pinned header
`vendor/linux/include/acpi/proc_cap_intel.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the sealed candidate and its
bound candidate snapshot, both independent review reports and attestations,
the frozen S012494 scope/symbol records, and the direct selected x86 consumer
contexts in `arch/x86/include/asm/acpi.h:116-141`,
`arch/x86/xen/enlighten_pv.c:344-358`,
`drivers/acpi/acpi_processor.c:572-590`, and
`drivers/acpi/processor_pdc.c:15-27`.  This was source inspection only.

### Parity P1 — C integer-expression contract: accepted, unresolved

Upstream lines 11-38 define the fifteen `ACPI_PROC_CAP_*` names as unsuffixed
integer constant expressions, including three `|` compositions.  The sealed
candidate instead publishes fifteen Rust `u32` constants.  The reopened direct
contexts demonstrate assignments and bitwise operations involving `u32`
storage, including `*cap &= ~(ACPI_PROC_CAP_C_C1_FFH |
ACPI_PROC_CAP_C_C2C3_FFH)` in `arch_acpi_set_proc_cap_bits()`.  They do not,
however, establish a frozen, all-selected-consumer mapping that authorizes
replacement of the C `int` expression/promotion domain with Rust `u32`,
especially for complement and mixed-expression contexts.

There is no S012494 entry in `rewrite/ABI.tsv` providing a target-specific C
macro bridge, integer representation, promotion rule, or Rust export/import
boundary.  The corresponding S012494 rows in `rewrite/SYMBOLS.tsv` remain
`PENDING_REVIEW`; the proposal's blanket `SOURCE_REVIEWED_VALUE` does not
supply that missing authority.  The applier cannot assume that all 354 frozen
header-closure consumers use the direct `u32` pattern observed in the cited
sites.  No source-preserving candidate change may be invented on that basis.

### Parity P2 — preprocessor guard and auditable candidate binding: accepted, unresolved

The pinned source's selected conditional and operative macro are the
preprocessor transition at lines 8-9 and 40: `#ifndef
__PROC_CAP_INTEL_H__`, `#define __PROC_CAP_INTEL_H__`, and `#endif`.  The
sealed Rust module has no C-visible guard bridge, and the frozen records do
not establish that the C preprocessor contract is absent from every selected
C/object boundary.  Its candidate snapshot is a wildcard summary rather than
a line-resolving binding for each selected macro and conditional.  Therefore
the proposal cannot close those pending records audibly.

### Rust review approval

The Rust review correctly identifies no intrinsic ownership, layout, or
runtime-state behavior in this declarative header.  That does not resolve the
independent cross-language macro-expression and preprocessor-interface
questions above.  The parity findings are therefore retained.

No candidate source was edited: the candidate is sealed and no exact
source-proven replacement mechanism is available.  No compiler, formatter,
analyzer, linker, test, runtime command, historical Rust source, or external
evidence was used.
