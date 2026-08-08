# Parity review — S000200 / slot 1

Reviewer: `parity_p02_s000200`  
Scope: `arch/arm64/include/asm/vncr_mapping.h` → `src/arch/arm64/include/asm/vncr_mapping.rs`  
Pinned Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`  
Verdict: **APPROVE — no parity findings.**

## Source evidence and coverage

Linux `arch/arm64/include/asm/vncr_mapping.h:10-113` contains 104
object-like, unconditional numeric macros.  They denote byte displacements in
the VNCR page (the Linux file comment at lines 2-4 explicitly makes the unit
bytes).  The Rust candidate has exactly one public `usize` constant with the
same identifier and hexadecimal value for every selected macro:

- `VNCR_VTTBR_EL2`, `VNCR_VTCR_EL2`, `VNCR_VMPIDR_EL2`,
  `VNCR_CNTVOFF_EL2`, `VNCR_HCR_EL2`, `VNCR_HSTR_EL2`,
  `VNCR_VPIDR_EL2`, `VNCR_TPIDR_EL2`, `VNCR_HCRX_EL2`,
  `VNCR_VNCR_EL2` (Linux lines 10-19).
- `VNCR_CPACR_EL1`, `VNCR_CONTEXTIDR_EL1`, `VNCR_SCTLR_EL1`,
  `VNCR_ACTLR_EL1`, `VNCR_TCR_EL1`, `VNCR_AFSR0_EL1`,
  `VNCR_AFSR1_EL1`, `VNCR_ESR_EL1`, `VNCR_MAIR_EL1`,
  `VNCR_AMAIR_EL1`, `VNCR_MDSCR_EL1`, `VNCR_SPSR_EL1`,
  `VNCR_CNTV_CVAL_EL0`, `VNCR_CNTV_CTL_EL0`, `VNCR_CNTP_CVAL_EL0`,
  `VNCR_CNTP_CTL_EL0`, `VNCR_SCXTNUM_EL1` (Linux lines 20-36).
- `VNCR_TFSR_EL1`, `VNCR_HDFGRTR2_EL2`, `VNCR_HDFGWTR2_EL2`,
  `VNCR_HFGRTR_EL2`, `VNCR_HFGWTR_EL2`, `VNCR_HFGITR_EL2`,
  `VNCR_HDFGRTR_EL2`, `VNCR_HDFGWTR_EL2`, `VNCR_ZCR_EL1`,
  `VNCR_HAFGRTR_EL2`, `VNCR_TTBR0_EL1`, `VNCR_TTBR1_EL1`,
  `VNCR_FAR_EL1`, `VNCR_ELR_EL1`, `VNCR_SP_EL1`, `VNCR_VBAR_EL1`,
  `VNCR_TCR2_EL1`, `VNCR_SCTLR2_EL1`, `VNCR_PIRE0_EL1`,
  `VNCR_PIR_EL1`, `VNCR_POR_EL1`, `VNCR_HFGRTR2_EL2`,
  `VNCR_HFGWTR2_EL2`, `VNCR_HFGITR2_EL2` (Linux lines 37-60).
- `VNCR_ICH_LR0_EL2` through `VNCR_ICH_LR15_EL2` in source order, and
  `VNCR_ICH_AP0R0_EL2` through `VNCR_ICH_AP0R3_EL2` in source order
  (Linux lines 61-80).
- `VNCR_ICH_AP1R0_EL2` through `VNCR_ICH_AP1R3_EL2`, `VNCR_ICH_HCR_EL2`,
  `VNCR_ICH_VMCR_EL2`, `VNCR_VDISR_EL2`, `VNCR_VSESR_EL2`,
  `VNCR_PMBLIMITR_EL1`, `VNCR_PMBPTR_EL1`, `VNCR_PMBSR_EL1`,
  `VNCR_PMSCR_EL1`, `VNCR_PMSEVFR_EL1`, `VNCR_PMSICR_EL1`,
  `VNCR_PMSIRR_EL1`, `VNCR_PMSLATFR_EL1`, `VNCR_PMSNEVFR_EL1`,
  `VNCR_PMSDSFR_EL1`, `VNCR_TRFCR_EL1` (Linux lines 81-99).
- `VNCR_MPAM1_EL1`, `VNCR_MPAMHCR_EL2`, `VNCR_MPAMVPMV_EL2`,
  `VNCR_MPAMVPM0_EL2` through `VNCR_MPAMVPM7_EL2`,
  `VNCR_ICH_HFGITR_EL2`, `VNCR_ICH_HFGRTR_EL2`, and
  `VNCR_ICH_HFGWTR_EL2` (Linux lines 100-113).

The source has only its include guard (`__ARM64_VNCR_MAPPING_H__`, Linux
lines 7-8 and 115); it contains no configuration condition, function, type,
static storage, linkable symbol, allocation, lock/RCU/refcount operation, or
error/cleanup path.  The Rust module likewise adds no executable mechanism,
storage, layout, or linkable symbol.  Its public constants retain all 104
source names and the aarch64-only scope recorded for S000200.

## Caller and arithmetic audit

The direct pinned consumer `arch/arm64/include/asm/kvm_host.h:438-451`
documents that these are byte offsets and expands `VNCR(r)` as
`__VNCR_START__ + ((VNCR_ ## r) / 8)`.  Its enum entries at lines 548-615 and
610-640 consume the named values to construct the sparse VNCR register-index
mapping.  Every source literal is non-negative, byte-aligned, and at most
`0xB20`; representing these aarch64 byte offsets as `usize` preserves each
literal and the required division/index arithmetic without a signedness,
width, overflow, evaluation-order, or ABI-visible change.  The C macros have
no side effects; Rust constants preserve that single-evaluation property.

No branding delta beyond the required immutable rewrite provenance was found.
No source-review uncertainty remains for this task.
