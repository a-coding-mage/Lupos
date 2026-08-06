# Parity review — S014514

Reviewed `src/include/linux/nfs_iostat.rs` against the complete pinned
`vendor/linux/include/linux/nfs_iostat.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, plus its selected x86_64 and
AArch64 scope/symbol/ABI/lifetime records and the immediate NFS consumer
context in `fs/nfs/iostat.h` and `fs/nfs/*`.

## Result

PASS — no parity findings.

## Checked mapping

- Provenance identifies the exact source path, pinned revision, `common`
  architecture membership, and task ID; it retains the required GPL-2.0-only
  SPDX identifier.
- `NFS_IOSTAT_VERS` remains the exact string literal `"1.1"`.
- `nfs_stat_bytecounters` remains an `int`-width (`i32`) counter-index domain;
  every enumerator is public, ordered, and has its exact C value:
  `NFSIOS_NORMALREADBYTES` through `NFSIOS_WRITEPAGES` are 0 through 7 and
  `__NFSIOS_BYTESMAX` is 8.
- `nfs_stat_eventcounters` likewise remains an `i32` counter-index domain;
  all 27 event enumerators are public, ordered, and exact from
  `NFSIOS_INODEREVALIDATE = 0` through `NFSIOS_PNFS_WRITE = 26`, with
  `__NFSIOS_COUNTSMAX = 27`.
- The immediate NFS consumers use these values only as the corresponding
  byte/event array indices (and the version as a formatting value); the alias
  plus constants preserves those values and their integer use.  This header
  defines no functions, objects, layouts, linkage, configuration branches, or
  architecture-specific declarations.
- The C include guard has no exported runtime/ABI content.  The destination is
  the single Rust module definition, so omitting the preprocessor guard does
  not omit a Linux declaration or selected conditional branch.

No source edits were made by this reviewer.
