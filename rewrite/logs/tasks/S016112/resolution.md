# Resolution — S016112

Applied against the complete pinned
`vendor/linux/include/uapi/linux/elf-em.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the common x86_64/AArch64 task
records, `src/include/uapi/linux/elf-em.rs`, and both independent reports.

## Parity review — accepted, no source change

The candidate retains all 49 `EM_*` object-like definitions in upstream source
order, with their original identifiers and values.  This includes the equal
`EM_MIPS_RS3_LE` and `EM_MIPS_RS4_BE` values, `EM_S390` following
`EM_X86_64`, and every hexadecimal value.  The UAPI SPDX expression and the
immutable source, revision, architecture, and task provenance match the pinned
source.  The file has no selected Kconfig branch, callable code, storage, or
linkage declaration.  The C include guard remains an upstream C-header
mechanism; its Rust module counterpart creates no runtime or ABI item.

## Rust review — accepted, no source change

Each source replacement list is an unsuffixed integer literal representable as
signed C `int` on both frozen targets, including the largest value, `0xbeef`.
The corresponding `pub const ...: i32` declarations preserve that source
constant-expression category and value without creating a C data object or
link symbol.  `Elf32_Half` and `Elf64_Half` `e_machine` fields are separate
consumer-side ABI conversions in `include/uapi/linux/elf.h`; this macro-only
header must not pre-narrow its signed C-int expressions.  The candidate has no
unsafe operation, layout-bearing item, allocation, ownership transfer,
lifetime, cleanup, lock, RCU/refcount, callback, panic path, or test.

## Final semantic-record closure

The 104 owned `SYMBOLS.tsv` rows (49 `EM_*` macros plus the include-guard macro
and opening/closing guard conditional for each approved architecture) are
closed as selected (`YES`) and `COMPLETE`.  The 50 machine identifiers per
architecture are unconditional source definitions; the guard conditional is
also present in both frozen configurations.  The guard's sole role is C
preprocessor inclusion control, so it has no Rust value or runtime analogue.

`ABI.tsv` has no S016112 row because this header declares no ABI-bearing type,
object, function, layout, alignment, calling convention, linkage symbol, or
FFI contract: **NOT_APPLICABLE**.  `LIFETIMES.tsv` likewise has no S016112 row
because it declares no storage, allocation, ownership/lifetime relation,
cleanup path, locking, RCU, refcount, or callback: **NOT_APPLICABLE**.
`DRIVER_ABI.tsv` has no S016112 row for the same macro-only reason:
**NOT_APPLICABLE**.  These conclusions follow directly from all definitions in
the pinned source and apply identically to frozen x86_64 and AArch64.

No compiler, formatter, build, linker, test, emulator, debugger, benchmark,
or runtime command was run.
