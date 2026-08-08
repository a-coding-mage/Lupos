# S000200 implementation — attempt 2

Pinned source: `arch/arm64/include/asm/vncr_mapping.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The source contains 104 `VNCR_*` object-like macro definitions.  This fresh
translation exports one `pub const` with the same identifier and hexadecimal
byte-displacement value for each definition.  Each constant has explicit type
`i32`, matching the source's unadorned C integer literals and preserving their
integer expression type rather than changing them to pointer-sized `usize`.

The C include guard has no Rust equivalent; no conditional source behavior is
introduced by the destination file.
