# Parity review — S016014

Reviewer: parity / P02 / slot 1 / Terra high

Verdict: **REJECT**

Reviewed source only:

- `vendor/linux/include/uapi/asm-generic/param.h`
- `src/include/uapi/asm-generic/param.rs`
- task records in frozen scope, symbol, ABI, lifetime, mapping, configuration, and header-closure metadata
- direct UAPI include hierarchy and selected x86_64/AArch64 consumers

## Findings

### P1 — generic defaults were converted into unconditional, fixed `i32` constants

Linux `param.h` lines 5–19 deliberately provides `__USER_HZ`, `HZ`, `EXEC_PAGESIZE`, and `NOGROUP` only when each macro has not already been defined.  `HZ` is specifically the macro expansion `__USER_HZ`, not an independently typed declaration.  The candidate’s lines 8, 11, 14, and 17 unconditionally publish fixed `i32` constants, so it cannot preserve a prior architecture or consumer definition, macro substitution, or the C expression’s contextual integer conversion.

This is a selected, material contract rather than dead conditional text.  The frozen aarch64 hierarchy defines `EXEC_PAGESIZE` as `65536` in `arch/arm64/include/uapi/asm/param.h:20` before including this generic header at line 22.  Therefore the selected aarch64 value is 65536, while candidate line 14 publishes 4096.  `include/uapi/linux/param.h:5` and `include/uapi/linux/acct.h:21` consume `<asm/param.h>`; `acct.h:124` further defines `AHZ` as `(HZ)`.  Header-closure evidence records 8,834 aarch64 and 2,887 x86_64 selected consumers.  No ABI or lifetime record authorizes replacing these macro contracts with fixed `i32` objects.

Required resolution: represent the guarded default/override relationship and preserve architecture-specific `EXEC_PAGESIZE` through the mapped UAPI interface.  Do not retain invented fixed Rust types as the macro contract; establish an exact per-architecture, consumer-visible representation from the pinned hierarchy and close the related `PENDING_REVIEW` symbol records.

### P2 — source guard and macro semantics are not represented as required selected branches

`SYMBOLS.tsv` inventories, for both architectures, the outer `_UAPI__ASM_GENERIC_PARAM_H` guard and all four inner `#ifndef` branches, plus the five operative macros.  Candidate lines 8–20 only encode final default values.  They omit every selected conditional branch and do not encode that `MAXHOSTNAMELEN` alone is unconditionally defined after the optional defaults.  Numeric equality of the defaults does not make the branching or UAPI namespace behavior equivalent.

Required resolution: map each inventory branch and macro contract explicitly, including its include-order behavior.  If Rust cannot express that C preprocessor contract within the leased source mapping, the applier must record the exact source-level reason and block rather than silently flatten it.

## Checks without findings

- The candidate provenance names the correct source, revision, task, and `common` architecture scope.
- The SPDX identifier matches the pinned generic header.  That header contains no additional copyright notice to retain.
- Default token values are otherwise numerically aligned: `__USER_HZ` 100, `HZ` -> `__USER_HZ`, generic `EXEC_PAGESIZE` 4096, `NOGROUP` `(-1)`, and `MAXHOSTNAMELEN` 64.
- Frozen configuration scheduler frequency (`CONFIG_HZ=1000` on x86_64; `CONFIG_HZ=250` on aarch64) is distinct from the UAPI default `__USER_HZ=100` and does not cure the missing conditional UAPI contract.

No compiler, formatter, linker, test, rust-analyzer diagnostic, or runtime command was run.  No source file was edited.
