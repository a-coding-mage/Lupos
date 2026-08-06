# Rust review — S013661

Reviewer role: Rust reviewer, slot 2 (source-only)

Verified before review: required branch `feat/bun-like-rewrite-test`; pinned
Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`; queue row S013661 is
`REVIEWING`, owned by P01; the mapped paths are
`include/linux/crc32poly.h` and `src/include/linux/crc32poly.rs`; the frozen
architecture set is `common` (selected for both x86_64 and aarch64).

## Findings

### F1 — `CRC32_POLY_BE` loses the C macro's required unsigned-expression conversion

**Severity: blocking**

The Linux header defines `CRC32_POLY_BE` as the unsuffixed hexadecimal integer
literal `0x04c11db7` (`vendor/linux/include/linux/crc32poly.h:9`).  It has
`int` type by itself, but it is a preprocessor replacement expression and
therefore participates in C's usual arithmetic conversions at each use site.
The selected common consumer makes this required behavior concrete:
`start_bunzip()` declares `c` as `unsigned int`
(`vendor/linux/lib/decompress_bunzip2.c:633-634`) and evaluates
`(c << 1) ^ (CRC32_POLY_BE)` in the CRC table loop
(`vendor/linux/lib/decompress_bunzip2.c:655-660`).  C converts the macro
literal to `unsigned int` before that XOR, preserving the intended 32-bit bit
pattern.

The candidate instead exports a fixed `i32` item,
`pub const CRC32_POLY_BE: i32 = 0x04c1_1db7;`
(`src/include/linux/crc32poly.rs:15-18`).  Rust performs no corresponding
implicit integer conversion for bitwise operators: an expression directly
equivalent to the selected caller's `u32 ^ CRC32_POLY_BE` is not available from
this item.  The explanatory text at candidate lines 15-17 describes C's
conversion but does not implement it.  A later caller-side cast would be an
extra, fallible per-use translation requirement and does not preserve this
header macro's expression semantics.  The same fixed-item design also cannot
provide the original macros' normal C conversion behavior in other integer
contexts.

No `unsafe`, layout, ownership, aliasing, or provenance issue was present in
the candidate beyond this finding.  No compiler, formatter, analyzer, build,
or test was run.
