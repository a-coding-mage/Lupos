# S016105 applier resolution

Reopened the complete pinned source `include/uapi/linux/dpll.h` at revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the candidate, both independent
reviews, and the in-tree DPLL declarations that consume the named enum tags.
This was source-only work; no compiler, formatter, test, or runtime command
was run.

## R1 — accepted and corrected

The Rust review correctly identified two distinct C interfaces.  The fourteen
`enum dpll_*` tags are retained as fourteen public aliases of `c_int`, matching
the selected Linux C `int` representation used by the tagged declarations.
Separately, all 132 translated `DPLL_*` / `__DPLL_*` macro and enumerator names
are `c_int` constant expressions: the aliases do not introduce a wrapper,
conversion, distinct Rust value type, or altered integer-promotion behavior.
The private same-named value-namespace helpers merely preserve the mechanical
initializer spelling; each returns its `c_int` argument unchanged and is not
an exported interface.  Every explicit value, implicit successor, flag mask,
and `__*_MAX - 1` expression remains the pinned source value.

This resolves the tag-ABI and enumerator-expression records for this task:
the tags name the selected `c_int` ABI and the enumerators are direct C-int
values, rather than transparent wrapper instances.

## R2 — accepted and corrected

`DPLL_FAMILY_NAME` and `DPLL_MCGRP_MONITOR` are now immutable static arrays of
`c_char`, respectively `[c_char; 5]` and `[c_char; 8]`, with their source bytes
and terminating NUL intact.  This retains the C string literal's fixed-array
contents and static storage.  Rust uses explicit `.as_ptr()` at a translated
use site to express C's ordinary-expression array-to-pointer conversion; that
pointer has the element type corresponding to C `char` and refers to the
static array rather than to a Rust slice reference.

## Final disposition

Both findings are resolved.  The parity review's exhaustive name/value result
remains valid after these representation corrections: fourteen enum tags and
all 132 translated names are present, and this UAPI header has no selected
configuration branch, function, struct, ownership, locking, or unsafe code.
