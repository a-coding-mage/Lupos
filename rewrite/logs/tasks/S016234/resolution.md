# Resolution — S016234 attempt 2

## Applier source review

I reopened the complete pinned source header
`vendor/linux/include/uapi/linux/major.h` at Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` and independently compared it
with `src/include/uapi/linux/major.rs`.

The source contains 140 `#define` directives: the preprocessing-only include
guard `_LINUX_MAJOR_H` plus 139 device-major definitions.  The Rust module has
the 139 same-named `pub const` definitions and intentionally has no item for
the C include guard.  Each source replacement is an unsuffixed decimal value
from 0 through 260 and therefore has `int` category on both frozen targets;
the corresponding Rust type is `i32`.  `HD_MAJOR` remains the named alias of
`IDE0_MAJOR`, and `UNIX98_PTY_SLAVE_MAJOR` remains the named
`UNIX98_PTY_MASTER_MAJOR + UNIX98_PTY_MAJOR_COUNT` expression.  No
architecture condition exists apart from the guard, so the immutable
provenance `architectures: common` is correct.

The sealed proposal, both review attestations, the current candidate, and the
Phase 0/queue bindings all match attempt 2 / P01.  No source discrepancy was
found; the candidate and prior evidence were not changed.

## Review dispositions

- Parity review (slot 1): APPROVE, with no findings. Disposition:
  `NOT_APPLICABLE`; its exhaustive macro/value, provenance, common-scope, and
  guard analysis agrees with the independent source review above.
- Rust semantics review (slot 2): APPROVE, with no findings. Disposition:
  `NOT_APPLICABLE`; `i32` value/category use, named alias/expression
  preservation, and absence of layout/ownership/unsafe behavior agree with
  the independent source review above.

No compiler, formatter, linker, test, runtime, or diagnostic command was
used.
