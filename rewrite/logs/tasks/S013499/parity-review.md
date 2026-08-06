# Parity review — S013499

Reviewer: parity_reviewer (`gpt-5.6-terra`, high)  
Pipeline: P02  
Scope reviewed: `include/linux/bcma/bcma_driver_arm_c9.h` → `src/include/linux/bcma/bcma_driver_arm_c9.rs`  
Pinned Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`

## Preconditions

- Required branch verified: `feat/bun-like-rewrite-test`.
- Queue row was `REVIEWING`, pipeline `P02`, task `S013499`; its mapped source and destination match the files above.
- `rewrite/SCOPE.tsv` classifies the header as `RUST_TRANSLATE`, architecture `common`, selected through the frozen x86_64 and aarch64 header closures. `rewrite/FILE_MAP.tsv` records those consumers, and `rewrite/SYMBOLS.tsv` selects the include guard plus all nine value macros for both architectures.
- `rewrite/PHASE0_IDENTITY.tsv` binds the pinned revision and frozen configurations to this queue.

## Findings

### P1 — SPDX identifier changed

`vendor/linux/include/linux/bcma/bcma_driver_arm_c9.h:1` is `/* SPDX-License-Identifier: GPL-2.0 */`, while candidate line 1 is `// SPDX-License-Identifier: GPL-2.0-only`. The rewrite protocol requires retention of SPDX identifiers; this is an unauthorized source-license identifier change. Restore the upstream identifier exactly.

### P1 — all nine unsuffixed C integer macros were changed to `u32` constants

Linux lines 6–14 define each selected value as an unsuffixed integer literal. Every candidate counterpart on lines 8–16 instead fixes the public Rust constant type to `u32`. These C literals fit in, and therefore have, the C `int` type; changing the constants to unsigned 32-bit changes expression typing (notably signedness/promotions and `~BCMA_DMU_CRU_USB2_CONTROL_USB_PLL_NDIV_MASK`). The latter is operative in `vendor/linux/drivers/phy/broadcom/phy-bcm-ns-usb2.c:66`; lines 51–67 also use the selected masks and shift values in arithmetic/bitwise expressions. Preserve the C literal type/usage semantics rather than imposing `u32` on each macro.

## Exhaustive comparison notes

- The candidate has all nine value-macro names: `BCMA_DMU_CRU_USB2_CONTROL`, both NDIV constants, both PDIV constants, `BCMA_DMU_CRU_CLKSET_KEY`, `BCMA_DMU_CRU_STRAPS_CTRL`, and both straps flags. Their numeric bit patterns match Linux lines 6–14 (underscore grouping only differs).
- Linux contains no functions, types, statics, ABI layout, conditional configuration branch, or executable logic in this header. The include guard at Linux lines 2–3 and 16 is structural; the Rust module is loaded once rather than preprocessor-included, so it has no separate Rust exported value to compare.
- The candidate provenance fields name the correct source path, frozen revision, `common` architecture membership, and task ID. No branding delta was found.

## Verdict

Rejected pending resolution of the two findings above. No source, build, formatter, test, debugger, or compiler diagnostic was run or used.
