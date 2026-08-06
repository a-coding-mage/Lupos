# Rust review — S016196 (slot 2)

Result: **REJECT — two required source corrections.**

Reviewed the complete pinned `include/uapi/linux/ioam6_genl.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the candidate, the frozen
`SYMBOLS.tsv`, `ABI.tsv`, and `LIFETIMES.tsv` records for S016196, and the
necessary pinned consumer `net/ipv6/ioam6.c`.  No build, formatter, test, or
runtime command was run.

## Findings

1. **High — named-enum enumerators and their derived macro no longer have C
   `int` expression semantics.**

   The C declarations at `include/uapi/linux/ioam6_genl.h:54-70` retain two
   enum tags for declarations, but each enumerator is an ordinary C enum
   constant expression and the macro
   `IOAM6_EVENT_ATTR_MAX` expands to the `int` expression
   `(__IOAM6_EVENT_ATTR_MAX - 1)`.  The candidate instead makes the tags
   nominal transparent wrapper structs (`src/include/uapi/linux/ioam6_genl.rs:16-27`)
   and gives all corresponding constants wrapper types (`:81-91`), requiring
   the private `.0` extraction in the public-max expression.  That changes
   their operation/conversion surface from C integer expressions to a distinct
   Rust type.  It also forces otherwise-unneeded conversions at the pinned
   consumers: `ioam6_event()` takes `enum ioam6_event_type`, passes it to the
   integer command argument of `genlmsg_put`, and switches on its enumerators
   (`net/ipv6/ioam6.c:634-662`); event attribute constants are passed to
   `nla_put_u16/u8/u32` (`:622-629`).

   Resolve by representing the named C enum tags as `c_int` aliases (subject
   to the frozen target ABI determination), while keeping every enumerator and
   `IOAM6_EVENT_ATTR_MAX` a `c_int` constant expression.  Do not retain a
   nominal wrapper merely for Rust type distinction: it is not the semantic
   type of a C enumerator.  The existing frozen ABI and lifetime entries remain
   `PENDING_REVIEW`; the applier must close the `c_int` representation decision
   for both targets with pinned evidence before `DONE`.

2. **Medium — both C string-literal macros were replaced by global objects,
   losing required macro expansion/array-initializer behavior.**

   `IOAM6_GENL_NAME` and `IOAM6_GENL_EV_GRP_NAME` are macros expanding to C
   string literals (`include/uapi/linux/ioam6_genl.h:12,52`), not declarations
   of addressable global arrays.  The candidate exports `pub static` arrays
   (`src/include/uapi/linux/ioam6_genl.rs:32-39,65-79`) and documents only an
   explicit `.as_ptr()` use.  This changes both identity/linkage and expression
   behavior, and it is already material in the pinned implementation: the
   macros occur as `.name` initializers in `net/ipv6/ioam6.c:614` and `:674`.
   C string-literal initialization supplies the characters and zero padding to
   those array fields; a `[c_char; 6]` or `[c_char; 13]` static is neither that
   initializer expression nor an implicit pointer decay in Rust.

   Resolve with a macro-/initializer-compatible representation and translate
   each pinned use according to its actual C context (array initialization
   versus pointer decay).  A bare public static plus a caller convention that
   exists only in a comment is insufficient for the selected UAPI macro
   surface.  Preserve the verified NUL-terminated byte sequences (`"IOAM6\\0"`
   and `"ioam6_events\\0"`) and do not add an exported object where upstream
   defines no object.

## Clean checks

- SPDX, source/revision/task provenance, and `common` architecture provenance
  match the queue row.
- All anonymous-enum/command/attribute values, sentinels, and integer max
  expressions have the expected numeric values; `255 * 4` remains an integer
  constant expression.
- The two literal byte counts and NUL terminators are correct.
- The source has no configuration conditional other than its include guard;
  no Rust conditionality was omitted.
- No `unsafe`, FFI declaration, layout-bearing aggregate, ownership, locking,
  allocation, panic path, or test was introduced.  The only ABI concern is the
  unresolved enum-representation assertion described above.
