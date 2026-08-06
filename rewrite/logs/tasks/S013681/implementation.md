# S013681 implementation

Translated `include/linux/decompress/unlzma.h` into the path-preserving
`src/include/linux/decompress/unlzma.rs` declaration only.

Evidence consulted:

- Pinned header: `vendor/linux/include/linux/decompress/unlzma.h:1-13`.
- Implementation contract: `vendor/linux/lib/decompress_unlzma.c:539-669`.
- Shared decompressor contract: `vendor/linux/include/linux/decompress/generic.h:5-34`.
- Frozen common selection: `rewrite/SCOPE.tsv` row `S013681`; both frozen
  configurations enable `CONFIG_DECOMPRESS_LZMA`.
- ABI/lifetime precedent finalized for the shared callback signature:
  `rewrite/ABI.tsv` and `rewrite/LIFETIMES.tsv` rows `S013677`.

The declaration uses the target C ABI and `c_long`/`c_ulong` (LP64 on both
approved targets). `c_char` preserves C `char` for the error string. `buf`,
`fill`, `flush`, `output`, and `posp` retain their nullable C forms. `error`
is a non-null function pointer because the implementation invokes it directly
on allocation, header, and corruption errors. No ownership is transferred;
all raw-pointer validity, buffer bounds, and callback lifetimes remain with
the caller for the call duration.
