# Resolution — S016053

Pinned source re-opened: `vendor/linux/include/uapi/linux/arm_sdei.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, in particular lines 6--8 and
28--37.  The selected AArch64 consumer at
`vendor/linux/drivers/firmware/arm_sdei.c:958--978` was also re-opened.  This
was a source-only application; no compiler, formatter, build, test, or runtime
command was run.

## P1 / R2 — fixed function signatures narrowed C macro conversions

Resolved.  `SDEI_1_0_FN`, `SDEI_VERSION_MAJOR`, `SDEI_VERSION_MINOR`, and
`SDEI_VERSION_VENDOR` now dispatch through sealed public traits whose associated
output types encode the frozen AArch64 LP64 integer promotions and usual
arithmetic conversions.  Each public wrapper consumes its operand exactly once.

- `SDEI_1_0_FN` has the header's `unsigned int` base category: promoted/narrow
  operands produce `u32`; a signed 64-bit-or-wider operand retains its signed
  category because it represents all `unsigned int` values; and an unsigned
  64-bit-or-wider operand retains its unsigned category.  `wrapping_add` gives
  the defined modular result for the unsigned category; signed overflow remains
  outside the source macro's defined behavior.
- The major/minor helpers retain the category resulting from their `int` masks,
  while the vendor helper separately preserves the conversion induced by the
  `unsigned int` vendor mask.  Thus the `u64 ver` expressions at the pinned
  caller continue to return `u64`, without rejecting other defined integer
  categories accepted by the source macro.

For promoted 32-bit inputs to the major/minor forms, the source shifts by 48 or
32 have undefined C behavior; the mapping retains the C result category but
does not claim a defined source result for those invalid shift counts.

## R1 — public object-macro literal categories changed

Resolved.  The six public version object macros now use their frozen C literal
categories exactly: all three shifts and the major/minor masks are `i32`
(`int`), while `SDEI_VERSION_VENDOR_MASK` is `u32` (`unsigned int`).  The two
function-number literals and all in-header derived function-number constants
remain `u32`, as required by their hexadecimal `unsigned int` source literals.
The remaining decimal and negative object macros remain `i32`.

All source symbols remain present with the original public spellings.  The
header contains no layout, linkage, ownership, locking, allocation, or unsafe
contract to resolve.

## Progressive-record closure

Completed exactly the 54 `S016053` rows in `rewrite/SYMBOLS.tsv`.  Their
selection/evidence fields now identify the source include-guard condition and
the exact `vendor/linux/include/uapi/linux/arm_sdei.h:<line>` to
`src/include/uapi/linux/arm_sdei.rs` mapping; no task row retains
`PENDING_REVIEW`.  There are no `S016053` rows in `rewrite/ABI.tsv`,
`rewrite/LIFETIMES.tsv`, or `rewrite/DRIVER_ABI.tsv`: this header declares only
preprocessor macros and its C include guard, not an ABI entity or a lifetime
owner.  No Phase 0 identity, frozen configuration, scope classification, or
queue immutable field was changed.
