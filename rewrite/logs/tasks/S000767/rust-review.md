# Rust review — S000767 (slot 2)

## Verdict

**REJECT — the two C enum declarations need an integer-domain representation,
and the frozen ABI/lifetime facts must be closed before this task can be
accepted.**

This was a manual, source-only review. I inspected the complete pinned
`vendor/linux/arch/x86/include/asm/xen/trace_types.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the candidate
`src/arch/x86/include/asm/xen/trace_types.rs`, the S000767 scope, symbol,
ABI, lifetime, and x86_64 header-closure/command metadata, and immediate
pinned usage in `include/trace/events/xen.h` and
`arch/x86/xen/multicalls.c`. No compiler, formatter, linker, test, runtime
tool, or historical Rust source was used.

## Findings

### R1 — High: nominal Rust enums impose invalid-discriminant invariants absent from both C enum types

`trace_types.h:5-16` declares ordinary, unforced C enums. Their compatible
integer objects can carry every value of that compatible integer type; the
four/three enumerators only name particular values. The candidate instead
uses fieldless Rust enums (`trace_types.rs:8-21`) whose valid values are only
the named discriminants. Any other C-compatible bit pattern arriving through
a trace field, raw storage, or retained C/driver boundary would be invalid as
the corresponding Rust enum.

This difference is observable in the immediate source context rather than
merely theoretical: `include/trace/events/xen.h:90-101` and `119-134` store
both enum values in trace records and deliberately print fallback `"??"` and
`"???"` strings for values outside the named enumerators. That code permits
the trace representation to be inspected even when it is not one of the
enumerated constants. A Rust nominal enum cannot faithfully hold such a
value.

Represent each tag with the exact ABI-proven compatible integer domain (for
example, a transparent integer newtype plus associated/flat named constants),
not a closed Rust enum. The replacement must retain all seven names, values
`0..=3` and `0..=2`, comparisons, trace-field storage, and the ability to
carry an otherwise unrecognised compatible integer value.

### R2 — High: both enum ABI records remain `PENDING_REVIEW`

The S000767 entries in `rewrite/ABI.tsv` for `enum xen_mc_flush_reason` and
`enum xen_mc_extend_args` still leave representation and ABI intent as
`PENDING_REVIEW`; the matching `rewrite/LIFETIMES.tsv` rows also remain
unclosed. The frozen x86_64 C command is available in the Phase 0 metadata,
but neither the candidate nor the records establish the C enum compatible
integer type, signedness, size, alignment, or by-value trace-field ABI.

This cannot be left implicit in `#[repr(C)]`: the C compatible integer type
for an unforced enum is implementation-defined. Before `DONE`, derive the
frozen x86_64 ABI from the pinned toolchain/configuration evidence, select a
Rust representation that explicitly preserves it, and close every S000767
ABI/lifetime semantic record with the source-grounded ownership and boundary
facts. If the exact compatible representation cannot be established, the task
must be blocked rather than accepted.

### R3 — Medium: the enum values are not freely copyable/comparable as their C counterparts are

Neither candidate enum derives `Copy`, `Clone`, `PartialEq`, or `Eq`. C enum
objects are freely copied and compared as compatible integers. The immediate
trace macros assign a passed enum to a record field and compare that field
multiple times (`include/trace/events/xen.h:95-101` and `126-134`); direct
Rust translation should not need a move-only workaround or borrow-based
substitute. The representation chosen to resolve R1/R2 must be explicitly
copyable and give integer-equivalent equality semantics, without concealing
invalid C values.

## Checks that passed

- The immutable provenance names the mapped Linux path, pinned revision,
  x86_64 architecture, and S000767 task correctly. The required source header
  and candidate SPDX identifiers are present; no branding delta was found.
- The candidate contains all source declarations in source order. The named
  enum values are correctly explicit and zero-based: flush `0..=3`, extend
  `0..=2`.
- `xen_mc_callback_fn_t` retains a nullable C-ABI function-pointer
  representation using `Option<unsafe extern "C" fn(*mut c_void)>`. The raw,
  mutable `void *` argument does not create a Rust reference or unsupported
  lifetime claim, and this declarative header introduces no unsafe block,
  allocation, synchronization operation, panic path, placeholder, Rust test,
  or configuration divergence.

No source files were edited by this reviewer.
