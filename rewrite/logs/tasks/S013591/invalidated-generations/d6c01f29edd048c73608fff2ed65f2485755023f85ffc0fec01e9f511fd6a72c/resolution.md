# S013591 resolution

## Reopened authority and frozen context

I reopened the complete pinned source
`vendor/linux/include/linux/circ_buf.h` at Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the current candidate, both
review reports, the frozen task row, and the materialized-cache equivalents in
`rewrite/phase0-bundles/`.  The task is `RUST_TRANSLATE`, common to x86_64 and
AArch64.  `metadata/header_closure.tsv` records exactly one Rust consumer on
each architecture: `kernel/events/ring_buffer.c`; the remaining consumers are
`LINUX_DRIVER_OBJECT` or `BUILD_METADATA`.

No compiler, formatter, linker, test, runtime command, or compiler-backed
diagnostic was run or used for this resolution.

## Finding dispositions

### PARITY-1 — `_TO_END` macros cached `size` (resolved by source change)

Accepted.  The pinned definitions at `include/linux/circ_buf.h:26-29` and
`:32-35` contain `size` twice: first in the `int end` initializer and again in
the mask used to initialize `int n`.  Their explicit one-access statement at
`:23-25` covers only `head` and `tail`.

`CIRC_CNT_TO_END!` now evaluates and retains `tail` for the first statement,
evaluates `size` for `end`, then evaluates `head` once and evaluates `size`
again for the mask.  `CIRC_SPACE_TO_END!` analogously retains `head` for the
first statement, evaluates `size` for `end`, then evaluates `tail` once and
evaluates `size` again for the mask.  This restores two `size` expression
evaluations at the two upstream statement-expression sites while retaining a
single expression evaluation of each protected index.  C leaves ordering of
operands within each individual arithmetic expression unspecified; the Rust
locals select a permitted order without moving `head` or `tail` across the
two C statement-expression initializers.

### PARITY-2 — generic C usual-arithmetic conversions (disproved as applying to this frozen Rust scope)

The reported mixed-type calls in `drivers/gpu/drm/msm/msm_perfcntr.c` are real
C calls, but they are not Rust call sites for this task.  Frozen `SCOPE.tsv`
classifies `drivers/gpu/drm/msm/msm_perfcntr.c` as `S004501`,
`LINUX_DRIVER_OBJECT`, AArch64, and `msm_rd.c` as `S004503`, also
`LINUX_DRIVER_OBJECT`.  Per the frozen scope, those sources remain original
Linux C objects and continue to consume the original pinned C macro; they do
not invoke `src/include/linux/circ_buf.rs`.

The header-closure evidence identifies the only translated consumer as
`kernel/events/ring_buffer.c` for both targets.  Its only uses are
`CIRC_SPACE` at `kernel/events/ring_buffer.c:147`, `:149`, and `:438`.
At `:142-149`, `head`, `tail`, and `data_size` are all declared `unsigned
long`; at `:378` and `include/linux/kernel/events/internal.h:132-135`,
`aux_head`, `aux_tail`, and `perf_aux_size(rb)` are all `unsigned long`.
Thus every operand within each translated macro invocation has the same C
integer type.  The `unsigned int size` at `ring_buffer.c:143` participates
only in the comparison after `CIRC_SPACE` has produced its `unsigned long`
result.  The retained target-width wrapping operations therefore model the
actual frozen translated calls without replacing the macro mechanism or
adding a new, unreviewed cross-integer conversion framework.

### RUST-1 — `_TO_END` macro evaluation count and ordering (resolved by source change)

Accepted for the same upstream evidence as PARITY-1.  The candidate no longer
binds `$size` to a local.  Its two direct `$size` uses occur after the same
respective index-localization points as the two upstream statement-expression
initializers.  `head` and `tail` remain raw expression values rather than Rust
references, so this change introduces no ownership, aliasing, `Drop`, or
interior-mutability contract.

## Closed task-local semantic records

- `struct circ_buf` is non-owning: the header supplies only `char *buf`,
  `int head`, and `int tail` (`circ_buf.h:9-13`), with no allocator,
  destructor, refcount, lock, RCU operation, or callback.  The Rust
  `#[repr(C)]` declaration preserves field order; `*mut u8` preserves the raw,
  non-exclusive pointer contract, and `c_int` preserves the two C `int`
  fields.  The frozen x86_64 and AArch64 `ring_buffer.c` commands both include
  `-funsigned-char`, establishing the `u8` pointee representation for this
  frozen configuration union.
- The four macros are compile-time expression mechanisms, not linked symbols
  or FFI functions.  They allocate nothing, establish no independent locking
  or RCU/refcount protocol, and require their caller to provide a power-of-two
  ring size exactly as the upstream arithmetic requires.  Rust uses explicit
  wrapping operations to retain the target-width unsigned counter arithmetic
  rather than debug/release-dependent overflow behavior.
- There are no configuration branches in the pinned header other than its
  include guard, and no branding delta.  The header's API is common to both
  frozen architectures.

All review findings now have a disposition and the task-local source evidence
closes the previously `PENDING_REVIEW` layout, ownership, lifetime, locking,
and macro-semantics conclusions for S013591.  The queue was not modified by
this applier.
