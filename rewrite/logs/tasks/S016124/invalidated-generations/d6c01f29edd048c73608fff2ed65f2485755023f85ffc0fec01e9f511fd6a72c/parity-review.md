# Parity review — S016124

Reviewed `src/include/uapi/linux/falloc.rs` against pinned
`vendor/linux/include/uapi/linux/falloc.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` for both selected architectures.

## Result

Accepted. No parity findings.

The source header contains only its include guard and nine unconditional UAPI
integer macros. The candidate exposes precisely these nine public constants,
with unchanged names and values:

- `FALLOC_FL_ALLOCATE_RANGE = 0x00`
- `FALLOC_FL_KEEP_SIZE = 0x01`
- `FALLOC_FL_PUNCH_HOLE = 0x02`
- `FALLOC_FL_NO_HIDE_STALE = 0x04`
- `FALLOC_FL_COLLAPSE_RANGE = 0x08`
- `FALLOC_FL_ZERO_RANGE = 0x10`
- `FALLOC_FL_INSERT_RANGE = 0x20`
- `FALLOC_FL_UNSHARE_RANGE = 0x40`
- `FALLOC_FL_WRITE_ZEROES = 0x80`

Each C macro is an unsuffixed integer literal and therefore has C `int` type;
the candidate uses `i32`, preserving the values and flag masks for the Linux
`int` fallocate mode interface on both x86_64 and AArch64. There are no source
types, functions, storage, configuration branches, ABI layouts, or linkage
definitions to translate. Rust has no textual-include mechanism, so omission
of the C include guard introduces no semantic difference.

The candidate retains the exact UAPI SPDX identifier
`GPL-2.0 WITH Linux-syscall-note`, correct pinned-source provenance, pinned
revision, common architecture scope, and task ID. No branding variance exists.
