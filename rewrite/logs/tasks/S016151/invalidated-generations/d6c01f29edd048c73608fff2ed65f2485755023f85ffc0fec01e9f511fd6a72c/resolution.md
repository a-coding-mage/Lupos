# Resolution — S016151

## Disposition of both reviews

Accepted without source changes.  The parity review and Rust-semantics review
each reported no findings, and an independent source recheck confirms that
`src/include/uapi/linux/hw_breakpoint.rs` is the complete faithful translation
of pinned `include/uapi/linux/hw_breakpoint.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

## Source recheck and final semantic mapping

- The complete upstream header contains exactly two anonymous enums and the
  ordinary `_UAPI_LINUX_HW_BREAKPOINT_H` include guard.  It contains no
  functions, objects, structs, storage, configuration conditionals, or
  architecture-specific branches.
- Anonymous C-enum enumerators have `int` type.  Both approved architectures
  have a 32-bit signed `int`, so the candidate's fourteen `pub const ...: i32`
  declarations preserve the source value type without inventing a
  storage-bearing Rust enum or an exported data symbol.
- Enum at upstream line 5 maps all eight values exactly:
  `HW_BREAKPOINT_LEN_1..HW_BREAKPOINT_LEN_8 = 1..8`.
- Enum at upstream line 16 maps all six values exactly:
  `HW_BREAKPOINT_EMPTY = 0`, `HW_BREAKPOINT_R = 1`, `HW_BREAKPOINT_W = 2`,
  `HW_BREAKPOINT_RW = HW_BREAKPOINT_R | HW_BREAKPOINT_W`,
  `HW_BREAKPOINT_X = 4`, and
  `HW_BREAKPOINT_INVALID = HW_BREAKPOINT_RW | HW_BREAKPOINT_X`.
  The two source bitwise expressions are retained as `i32` expressions;
  their values are respectively 3 and 7, with no overflow, promotion, or
  signedness difference.
- The Rust module path is the frozen one-to-one mapping for this UAPI header.
  The C include guard is a preprocessing multiple-inclusion mechanism, not a
  runtime or UAPI value, and therefore has no Rust item counterpart.
- `SCOPE.tsv` records this header as `common`, selected through header closure
  for 9 AArch64 and 27 x86_64 consumers.  Pinned AArch64 and x86 breakpoint
  paths use the constants in exact `switch` cases, assignments, and bit masks;
  e.g. AArch64 converts `R`, `W`, `RW`, `X` and lengths 1--8, while x86 maps
  lengths 1, 2, 4, and 8 and combines `W | R`.  These uses confirm the public
  integer-constant semantics.  They do not add architecture-specific content
  to this common UAPI header.

## Closed Phase-0 pending semantic facts

For `anonymous_enum@5` and `anonymous_enum@16` on both AArch64 and x86_64:

- **Symbols/conditions:** final mapping is the fourteen public constants
  listed above; the guard is not an operative Rust configuration branch.
- **ABI:** no aggregate layout, alignment, linkage, calling convention, or
  exported storage exists.  Each enumerator is a signed 32-bit integer value;
  downstream widening to `perf_event_attr.bp_type` (`__u32`) or `bp_len`
  (`__u64`) is exact for these non-negative values.
- **Ownership/lifetime/locking/RCU/refcounting:** not applicable.  The header
  declares constants only and introduces no allocation, pointer, aliasing,
  lifetime, synchronization, or cleanup behavior.
- **Branding/UAPI:** names, values, SPDX expression, public header scope, and
  Linux UAPI semantics are unchanged; no branding allowlist entry is used.

No compiler, formatter, linker, test, runtime command, or diagnostic was run.
