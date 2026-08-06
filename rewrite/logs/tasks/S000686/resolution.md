# Resolution — S000686

I independently reopened the complete pinned
`vendor/linux/arch/x86/include/asm/shared/tdx_errno.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, its frozen x86_64 scope and
compiler-target records, the candidate, and both review reports.

## Review dispositions

1. **Parity P1 — accepted and corrected.** `TDX_OPERAND_ID_RCX`,
   `TDX_OPERAND_ID_TDR`, `TDX_OPERAND_ID_SEPT`, and
   `TDX_OPERAND_ID_TD_EPOCH` are unsuffixed source literals on lines 35--38.
   Each fits the frozen C `int` category, so the candidate's `u32` type was
   replaced with `i32`. Their values remain respectively `0x01`, `0x80`,
   `0x92`, and `0xa9`; the bits-31:0 comment describes error-detail placement,
   not an unsigned literal type.
2. **Rust R1 — accepted and corrected.** The same `i32` mapping preserves the
   source literal category and leaves any required C usual-arithmetic
   conversion to the translated use site. The frozen target is
   `x86_64-linux-gnu`, and its pinned LLVM 19 compile-command family uses
   GNU11 C. No out-of-file source use was found in the pinned x86/header
   closure search; this header itself supplies all four definitions.
3. **All other review checks — accepted.** The status mask and nineteen
   SEAMCALL status codes retain their exact `ULL` 64-bit values as `u64`.
   Every macro name, value, immutable provenance field, architecture, and
   SPDX identifier remains unchanged.

## Final semantic records

- The `S000686` scope row and all 27 `SYMBOLS.tsv` records are `COMPLETE`.
  The include guard is guard-only; every remaining macro is unconditional and
  selected. The 20 `ULL` macros map to `u64`; the four unsuffixed operand IDs
  map to `i32`.
- `ABI.tsv`, `LIFETIMES.tsv`, and `DRIVER_ABI.tsv` have no S000686 record.
  This macro-only header has no object, function, type, C linkage/export,
  layout, alignment, calling convention, storage, ownership, lifetime,
  cleanup, locking, RCU, refcount, callback, allocation, or unsafe boundary.
- No branding delta, placeholder, or Rust test was introduced.

No compiler, formatter, build, linker, test, emulator, debugger, benchmark,
or runtime command was run. `DONE` will mean only source-pipeline completion,
not compilation or testing.
