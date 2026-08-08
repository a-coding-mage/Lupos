# Parity review — S000200 / P02 / attempt 2 / slot 1

Reviewer: `parity_clean_p02_s000200` (`gpt-5.6-terra`, high)

Scope reviewed: pinned `vendor/linux/arch/arm64/include/asm/vncr_mapping.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, current candidate
`src/arch/arm64/include/asm/vncr_mapping.rs`, and the current S000200 queue/scope/symbol records.
No historical Rust source, prior review report, compiler, formatter, linker,
test, rust-analyzer diagnostic, or runtime output was inspected or used.

Frozen-artifact hashes supplied for this review:

- Phase 0 identity: `0123af9e96b58af4e98b91a2de5608d36b63c67864b9041d3340587f3dbe40d2`
- Queue: `cfa8dcf0d81c408de38d4202fe1ce9d9e5e301d1e67439c9530da94b1e64ee3f`
- Scope: `b83349e6731e17e5da5e04a0ea053487e8ac8d9269538dbdb367d83f77b17e0`
- Symbols: `7484d7b0dd80f45e18d726b04752827fe28555cc5c1af0e127948380e6688abf`
- ABI: `ae0caca34fe9b6eb8097958d2fbb8d2b6a1fad60f91f3c2b8d948c43dbfcee39`
- Lifetimes: `0e7e60940dd21c28f3d10965325f70644fc000495a2d61984437dec666da93d8`

## Result

No parity findings.

## Evidence

- The pinned header contains exactly 104 operative `VNCR_*` object-like macro
  definitions (lines 10–113). The candidate contains exactly 104 corresponding
  `pub const` definitions. A source-text tuple comparison of identifier and
  hexadecimal token found no missing name, extra name, or unequal value.
  This covers every selected macro from `VNCR_VTTBR_EL2` through
  `VNCR_ICH_HFGWTR_EL2`, including the discontinuous offsets and the full
  `VNCR_ICH_LR0_EL2`–`VNCR_ICH_LR15_EL2`, `VNCR_ICH_AP0R0_EL2`–
  `VNCR_ICH_AP1R3_EL2`, and `VNCR_MPAMVPM0_EL2`–`VNCR_MPAMVPM7_EL2` ranges.
- For every Linux macro, the literal is an unsuffixed hexadecimal integer no
  greater than `0xB20`; in the pinned AArch64 C context it has `int` type.
  The candidate explicitly uses `i32` for every matching Rust constant, which
  preserves that literal width and signedness. The header has no functions,
  calls, linkage declarations, aggregate layout, allocation, locking, or
  lifetime mechanism; therefore there is no file-local caller/control-flow
  behavior to translate.
- The candidate preserves each macro spelling and value and adds only the
  required immutable provenance plus a descriptive Rust documentation comment.
  It introduces no Linux-to-Lupos branding delta, generated state, stub,
  wrapper, FFI symbol, ABI layout, or behavioral mechanism.
- Current S000200 has no direct task dependency. `ABI.tsv` and
  `LIFETIMES.tsv` contain no S000200 records; the selected symbol records are
  the header guard and 104 operative macros. Any remaining manifest
  `PENDING_REVIEW` closure is an applier responsibility and is not a candidate
  source mismatch.

Review conclusion: the current candidate is source-parity clean for the
pinned header and frozen S000200 scope.
