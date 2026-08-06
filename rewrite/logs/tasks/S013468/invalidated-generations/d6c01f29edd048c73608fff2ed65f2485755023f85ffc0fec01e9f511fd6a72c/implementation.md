# S013468 implementation

- Leased task: `include/linux/asn1.h` to `src/include/linux/asn1.rs`.
- Pinned Linux revision verified as `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- The complete header is unconditional for the frozen common x86_64/aarch64 scope and contains three C enum tags plus three integer macros.
- `asn1_class`, `asn1_method`, and `asn1_tag` are represented as C `int` aliases. This preserves the C enumerators' integer-expression behavior used by selected callers (tag-byte shifts, masks, comparisons, and bitwise OR) without imposing Rust enum validity restrictions on values crossing C-compatible boundaries.
- All enumerators, reserved tag gaps, macro values, SPDX/copyright notice, and immutable common-architecture provenance are present.
- No build, formatter, compiler, linker, test, or runtime command was run.
