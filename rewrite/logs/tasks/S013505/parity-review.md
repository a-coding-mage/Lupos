# Parity review — S013505, attempt 1, slot 1

**Reviewer:** `parity_p02_s013505`  
**Scope:** `include/linux/bcma/bcma_regs.h` → `src/include/linux/bcma/bcma_regs.rs`  
**Pinned Linux revision:** `425f94c2954b1fe80ebdbf9b29854e89750355df`  
**Result:** APPROVE

## Source evidence reviewed

- The complete pinned header is a guarded collection of 74 object-like macros: 56 unsuffixed integer literals, 17 `U`-suffixed 32-bit address literals, and `BCMA_SOC_PCI_MEM_SZ` as the integer expression `(64 * 1024 * 1024)` (`vendor/linux/include/linux/bcma/bcma_regs.h:2-104`). It contains no functions, types, statics, executable branches, allocation, locking, ABI linkage, or error paths.
- `rewrite/SCOPE.tsv` maps this selected common header to the candidate path for both frozen architectures. The S013505 symbol inventory enumerates the header guard, all macro names, and both delimiter conditionals; the task has no S013505 ABI or lifetime records.
- Candidate inspection confirms one public Rust constant for every non-guard source macro, with the original identifier and numerical value. The 17 source `U`-suffixed literals are `u32`; every unsuffixed literal, including the preserved `64 * 1024 * 1024` expression, is `i32`. The two documented reversed-bit names retain their source aliases.

## Parity disposition

The Rust module has no replacement mechanism, wrapper, stub, unauthorized branding, or behavior beyond the source constant definitions. Rust module loading provides the source header guard's one-definition purpose; there are no conditional configuration branches in the pinned header apart from that guard. All source macros, aliases, values, and C integer-category distinctions represented by the literals are present in the candidate.

No parity findings.
