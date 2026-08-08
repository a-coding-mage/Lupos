# Parity review — S016011, attempt 2, slot 1

## Result

APPROVE. No parity findings.

## Reviewed material

- Pinned Linux source: `vendor/linux/include/uapi/asm-generic/mman-common.h` at `425f94c2954b1fe80ebdbf9b29854e89750355df` (the value in `vendor/linux.SHA`).
- Candidate: `src/include/uapi/asm-generic/mman-common.rs` and the current `candidate.diff`.
- Frozen task records: `TRANSLATION_TASKS.tsv`, `SCOPE.tsv`, `FILE_MAP.tsv`, and `SYMBOLS.tsv` for S016011. They select this unconditional common header for both x86_64 and AArch64 and inventory its include guard, 53 value macros, and closing conditional.
- Narrow pinned contexts: `include/uapi/asm-generic/mman.h`, x86_64 and AArch64 `include/uapi/asm/mman.h`, `include/uapi/linux/mman.h`, `include/linux/mman.h`, x86 `arch_set_user_pkey_access`, and AArch64 `arch_set_user_pkey_access`.

## Evidence

- Every one of the Linux header's 53 value macros has exactly one same-named candidate `pub const`; the numeric values, hexadecimal spellings where material, and the `i32` type agree with the C unsuffixed integer-constant type on both selected LP64 targets. Every literal is representable by signed 32-bit `int`; none crosses a sign, width, or overflow boundary.
- Linux symbol `PKEY_ACCESS_MASK` remains an expression of `PKEY_DISABLE_ACCESS | PKEY_DISABLE_WRITE`, yielding `0x3`; it is not replaced by an unrelated hardcoded value. The pinned AArch64 `asm/mman.h` explicitly undefines and replaces this generic macro with the four-bit AArch64 expression. The candidate faithfully supplies only the generic default in this file, leaving that architecture-specific replacement to its mapped architecture header. Pinned x86 and AArch64 callers consume the individual PKEY values through integer bit tests, matching the generic `i32` source value domain before their caller-level conversions to `unsigned long` parameters.
- The Linux `__ASM_GENERIC_MMAN_COMMON_H` include guard is an unconditional textual-inclusion guard, not a runtime or configuration branch. The candidate has no `cfg` branch and is a single Rust module source; Rust module inclusion provides the corresponding module-once behavior without exporting the C preprocessor sentinel. The selected conditional inventory is therefore covered without changing the header's value surface.
- The candidate retains the exact UAPI SPDX identifier, upstream attribution, Linux source path, pinned revision, `common` architecture scope, and S016011 provenance. It adds no branding, exported ABI object, layout, linkage, allocation, locking, error path, stub, test, or conditional behavior.

No compiler, formatter, test, runtime command, or compiler-backed diagnostic was invoked.
