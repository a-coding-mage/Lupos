# Resolution — S016124

## Applier verification

Reopened the complete pinned source
`vendor/linux/include/uapi/linux/falloc.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` and independently compared it
with `src/include/uapi/linux/falloc.rs`.

The source has exactly nine unconditional UAPI macro definitions and no
functions, types, storage, configuration-dependent branches, ABI layouts, or
linkage declarations.  The candidate retains each public name and the exact
unsuffixed C `int` value, represented by the corresponding 32-bit Rust `i32`
constant:

- `FALLOC_FL_ALLOCATE_RANGE = 0x00`
- `FALLOC_FL_KEEP_SIZE = 0x01`
- `FALLOC_FL_PUNCH_HOLE = 0x02`
- `FALLOC_FL_NO_HIDE_STALE = 0x04`
- `FALLOC_FL_COLLAPSE_RANGE = 0x08`
- `FALLOC_FL_ZERO_RANGE = 0x10`
- `FALLOC_FL_INSERT_RANGE = 0x20`
- `FALLOC_FL_UNSHARE_RANGE = 0x40`
- `FALLOC_FL_WRITE_ZEROES = 0x80`

`i32` preserves the C `int` values and flag bits on both frozen x86_64 and
AArch64 targets.  The C include guard is a preprocessor-only multiple-include
mechanism and has no Rust module-level semantic counterpart.  The candidate
also has the exact UAPI SPDX identifier, source path, pinned revision, common
architecture scope, and task ID.  No branding delta exists.

## Review dispositions

- Parity review: accepted; independently confirmed.  No source change needed.
- Rust review: accepted; independently confirmed.  No source change needed.

All task-local semantic records are resolved as not applicable beyond the
nine constant mappings.  No source edit was necessary.
