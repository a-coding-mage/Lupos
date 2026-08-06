# Rust semantics review — S016371

Reviewed `vendor/linux/include/uapi/linux/seg6_genl.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/uapi/linux/seg6_genl.rs`, with the selected consumer
`vendor/linux/net/ipv6/seg6.c`.

## Finding R1 — `SEG6_GENL_NAME` loses C literal storage and decay semantics

**Severity: major.**  C `SEG6_GENL_NAME` expands to the string literal
`"SEG6"`: an array of five `char` elements with static storage duration.  At
the selected consumer (`net/ipv6/seg6.c:496`), that literal is used to
initialize `struct genl_family.name`, whose type is `char[GENL_NAMSIZ]`
(`include/net/genetlink.h:78-82`).  C's string-literal aggregate
initialization supplies the four bytes and terminator and zero-initializes the
remainder.  In pointer-expression contexts the same macro decays to a pointer
to static literal storage.

The candidate's `pub const SEG6_GENL_NAME: [c_char; 5]` is a by-value Rust
constant.  Each use denotes a copied five-element value, not storage with a
stable FFI address; taking a pointer from a temporary use does not provide the
C literal lifetime.  It also cannot initialize a Rust `[c_char;
GENL_NAMSIZ]` field directly, so it does not model the selected `.name =
SEG6_GENL_NAME` aggregate-initializer use.  The comment that it retains the
NUL "for C consumers" is therefore not true: a Rust `const` exports neither
a C preprocessor macro nor an FFI object/pointer.

**Required resolution:** replace or supplement the by-value representation
with an immutable static/`'static` backing C string for pointer uses, and make
the Rust translation of the `genl_family.name` initializer explicitly produce
the exact zero-padded `GENL_NAMSIZ` array.  Record the resulting source-level
mapping for the macro's aggregate and pointer contexts; do not rely on the
five-element `const` as an FFI representation.

## Checked items

- The two anonymous C enum declarations introduce no named enum type.  Their
  enumerators are C `int` constant expressions, and all values fit `c_int` on
  both selected architectures.  The candidate preserves their names and
  ordinals: attributes `0..8`, commands `0..5`.
- `SEG6_ATTR_MAX` and `SEG6_CMD_MAX` preserve the parenthesized C arithmetic
  result (`7` and `4`) at `c_int` width; no promotion, truncation, invalid-value
  restriction, or enum-object ABI is introduced.
- ASCII bytes and the terminating zero are representable for either selected
  target's `c_char` signedness.  No `unsafe`, layout-bearing type, hidden
  assertion, test configuration, panic, or unauthorized branding is present.

Source-only review; no compiler, formatter, test, or runtime command was run.
