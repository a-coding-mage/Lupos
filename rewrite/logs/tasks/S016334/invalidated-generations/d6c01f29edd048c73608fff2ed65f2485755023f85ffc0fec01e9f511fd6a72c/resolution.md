# Applier resolution — S016334

Task `S016334` translates `include/uapi/linux/posix_acl.h` to
`src/include/uapi/linux/posix_acl.rs` for the frozen common x86_64 and AArch64
scope at pinned Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.

## Independent source recheck

I reopened all 40 lines of the pinned UAPI header, the candidate, both frozen
configuration records, the task scope/symbol records, and both independent
reviews. Lines 18--19 and 40 are the unconditional C include guard only; they
create no Rust item, storage, linkage, ABI, or runtime effect. The remaining
twelve payload macros are present in the candidate with their exact public
names, order, values, and signed C-`int` expression category: the `-1`
sentinel; the two ACL types; six entry tags; and three permission bits.

The frozen x86_64 and AArch64 targets both use a signed 32-bit C `int` for
these unsuffixed integer constant expressions. The candidate's public `i32`
constants therefore preserve each source value, including
`ACL_UNDEFINED_ID == -1`; no target-dependent conversion, type, layout,
function, object, configuration branch, ownership, lifetime, locking, RCU,
refcount, allocation, cleanup, or driver-ABI behavior is declared by this
header.

## Review dispositions

1. Parity review: accepted. The independent recheck confirms its exhaustive
   twelve-name/value inventory and its conclusion of no source finding.
2. Rust review: accepted. Its statement that the source has “eleven
   object-like macros” is a clerical count error: it subsequently enumerates
   all twelve payload values and the candidate has all twelve. It does not
   identify a source, Rust, ABI, or safety defect. The candidate contains no
   unsafe code, ownership boundary, FFI declaration, panic path, test, or
   substitute mechanism.

## Semantic-record closure

All 30 S016334 `SYMBOLS.tsv` records are now `COMPLETE`: for each frozen
architecture, the two guard conditionals and guard definition record their
no-Rust-item treatment, and each of the twelve payload macros records its
exact public `i32` value. The S016334 `SCOPE.tsv` semantic status is likewise
`COMPLETE`. `ABI.tsv`, `LIFETIMES.tsv`, `DRIVER_ABI.tsv`, and `BLOCKERS.tsv`
contain no S016334 row, which is correct for this passive constants-only UAPI
header; no task-local `PENDING_REVIEW` remains.

All five required task evidence files exist. No compiler, formatter, linker,
test, emulator, debugger, runtime command, or benchmark was run.
