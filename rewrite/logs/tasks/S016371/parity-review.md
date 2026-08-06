# Parity review — S016371

Reviewed `vendor/linux/include/uapi/linux/seg6_genl.h` at pinned revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/uapi/linux/seg6_genl.rs`, plus its selected `net/ipv6/seg6.c`
consumer context.  This was a source-only review; no compiler, formatter,
linker, test, or diagnostic tool was run.

## Finding P1 — `SEG6_GENL_NAME` lacks the macro's stable string storage

The C macro at `include/uapi/linux/seg6_genl.h:5` expands to the string literal
`"SEG6"`: a five-byte, NUL-terminated `char` array with static storage that
decays to a pointer when used for `seg6_genl_family.name` in
`net/ipv6/seg6.c:496`.  The candidate defines `SEG6_GENL_NAME` as a Rust
`pub const [c_char; 5]`.  A Rust `const` is an inlined value, not one
addressable static object, so it cannot faithfully provide the stable pointer
semantics of the source macro at the generic-netlink family initializer.

Replace it with a public NUL-terminated `static [c_char; 5]` (and use
`.as_ptr()` at the translated C pointer-decay use site), retaining exactly the
five bytes `S`, `E`, `G`, `6`, `\\0`.

## Verified parity

- SPDX license expression and all immutable provenance fields match the pinned
  source and task row; the source header contains no additional copyright
  notice to carry over.
- `SEG6_GENL_VERSION` is the C `int` constant `0x1`.
- Both anonymous enum enumerator sequences have C-`int` values and ordinals:
  attributes `0..8` and commands `0..5`; both public derived `*_MAX` values
  remain respectively `7` and `4` via the source expressions.
- The candidate preserves every UAPI public enumerator spelling, both reserved
  double-underscore maximum identifiers, and the terminating NUL in the name.
- The header has no configuration-dependent branch; its C include guard has no
  separate Rust conditional equivalent.

## Disposition

One source-parity finding remains for the applier to resolve.  No source files
were modified by this reviewer.
