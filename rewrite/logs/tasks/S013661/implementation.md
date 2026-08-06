# S013661 implementation

Translated `include/linux/crc32poly.h` at Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` into the path-preserving
`src/include/linux/crc32poly.rs` for the frozen common (`x86_64`, `aarch64`)
task.

The source header contains only three object-like polynomial macros. Their
values and C literal types are preserved: `CRC32_POLY_LE` and
`CRC32C_POLY_LE` are hexadecimal literals that require `unsigned int` and map
to `u32`; `CRC32_POLY_BE` fits `int` and maps to `i32`. The selected
`lib/decompress_bunzip2.c` use combines the latter with an `unsigned int` CRC,
so C applies the usual arithmetic conversion to `unsigned int` at that call
site; no CRC algorithm or callable interface is introduced here.

No conditional configuration branch, storage, ABI item, ownership relation,
or synchronization behavior exists in this header.
