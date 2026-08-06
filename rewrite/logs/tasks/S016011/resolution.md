# Applier resolution — S016011

Reviewed the complete pinned `include/uapi/asm-generic/mman-common.h` at
Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`, the candidate
`src/include/uapi/asm-generic/mman-common.rs`, and both independent review
reports.

## Review dispositions

| Report | Finding | Disposition |
| --- | --- | --- |
| Parity review | No findings | Accepted. The complete generic UAPI macro definition set is represented. |
| Rust review | No findings | Accepted. This constant-only UAPI header creates no ownership, layout, FFI, synchronization, or unsafe boundary. |

## Independent applier checks

- `vendor/linux.SHA`, the pinned checkout `HEAD`, and immutable destination provenance all identify `425f94c2954b1fe80ebdbf9b29854e89750355df` and task `S016011` for the common architecture scope.
- A direct identifier audit found exactly 53 upstream defined UAPI macros and exactly 53 Rust public constants, with an identical ordered identifier set. Literal values preserve the source `int` values, and `PKEY_ACCESS_MASK` remains the bitwise-OR expression over `PKEY_DISABLE_ACCESS` and `PKEY_DISABLE_WRITE`.
- The only C conditional is the include guard and has no Rust item analogue. The source contains no functions, storage, aggregate layouts, linkage declarations, ownership rules, locking, or configuration-selected branch to translate.
- The AArch64 UAPI parent later undefines and replaces `PKEY_ACCESS_MASK`; this is architecture-parent behavior, not a condition or alteration in the generic header. The generic candidate correctly retains the unmodified source definition.
- The 112 Phase-0 `SYMBOLS.tsv` records for this header (include-guard conditions and both-architecture macro inventory) are resolved by the direct one-to-one mapping. There are no S016011 records in `ABI.tsv`, `LIFETIMES.tsv`, `DRIVER_ABI.tsv`, or `BLOCKERS.tsv`.
- The destination retains the UAPI SPDX identifier and required immutable provenance, and contains no project-authored Rust test, placeholder, unsafe operation, FFI item, or unauthorized branding.

No source change is required. The candidate is accepted as the full fresh
translation of the selected generic UAPI header. No build, formatting, test,
linker, or runtime command was run.
