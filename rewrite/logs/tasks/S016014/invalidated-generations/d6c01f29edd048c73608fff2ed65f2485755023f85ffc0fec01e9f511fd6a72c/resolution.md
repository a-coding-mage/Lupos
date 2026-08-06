# Resolution — S016014

Applier: P02 / Terra high

## Result: BLOCKED

The candidate is rejected and no Rust source change is accepted.  The task is
blocked rather than replacing the C preprocessor contract with an invented
Rust facade.

### Finding dispositions

#### P1 / R1 — guarded defaults and the arm64 override: confirmed, unresolved

`include/uapi/asm-generic/param.h` is not a declaration of five unconditional
objects.  It uses separate `#ifndef` tests for `__USER_HZ`, `HZ`,
`EXEC_PAGESIZE`, and `NOGROUP` (lines 5--19), so every one is an
include-order-dependent default.  `MAXHOSTNAMELEN` alone is unconditionally
defined at line 21.

The selected arm64 provider
`arch/arm64/include/uapi/asm/param.h:20--22` defines `EXEC_PAGESIZE` as 65536
*before* it includes this generic header.  The generic branch consequently
does not define that macro for arm64.  The generic 4096 replacement token is
the selected x86_64/default result.  A `pub const EXEC_PAGESIZE: i32 = 4096`
in the common destination is therefore incorrect for arm64 and cannot be
shadowed by a later Rust item with the same module namespace.

The frozen hierarchy has no pre-existing Rust macro/conditional-definition
mechanism that lets a common mapped file conditionally emit a public item based
on an earlier architecture/header definition.  The only mapped arm64 wrapper
is the later task S000217, which depends on this task; it cannot repair an
already unconditional common item without an unapproved shared namespace or
module-index design.  `rewrite/SCOPE.tsv` records 8,834 arm64 and 2,887 x86_64
header-closure consumers of this source.

#### P2 / R2 — UAPI `HZ` versus internal `HZ`: confirmed, unresolved

The generic UAPI default is `HZ -> __USER_HZ -> 100`.  Separately,
`include/asm-generic/param.h:5--10` includes this header, undefines `HZ`, and
redefines it as `CONFIG_HZ`; the frozen values are 1000 on x86_64
(`rewrite/configs/x86_64/frozen.config:469--470`) and 250 on arm64
(`rewrite/configs/aarch64/frozen.config:470--473`).  A common public Rust
`HZ` item has neither C macro expansion nor a `#undef`/redefinition operation,
so it would wrongly become the apparent internal value unless a later,
frozen architecture/configuration-aware macro namespace is specified.

#### R3 — contextual macro expression semantics: confirmed, unresolved

The source supplies replacement tokens, not typed/linkable objects.  The
unsuffixed literals and `(-1)` have C `int` type in their immediate expressions
on the pinned targets, but macro expansion remains subject to every consumer's
ordinary C conversions.  `HZ` expands through another macro and `NOGROUP` may
be converted in an unsigned consumer expression.  Fixed `i32` Rust constants
change both the namespace and conversion contract.  The header has no
structure, union, enum, function, linkage, layout, ownership, or lifetime
object that could provide an alternative ABI representation.

### Manifest closure

All S016014 `SYMBOLS.tsv` rows 320995--321026 remain `PENDING_REVIEW` because
the selected outer include guard, four inner `#ifndef` branches, five macro
contracts, their architecture-dependent selection, and their source namespace
cannot be closed by this one-file Rust mapping.  There are no S016014 rows in
`ABI.tsv`, `LIFETIMES.tsv`, or `DRIVER_ABI.tsv`; the absence is consistent with
this macro-only header, but does not resolve its namespace behavior.

### Required unblock evidence

A reviewed, frozen Rust-side representation must first specify how the mapped
UAPI hierarchy preserves: (1) a prior architecture/header definition winning
over each generic default; (2) arm64's 65536 versus x86_64/default 4096
`EXEC_PAGESIZE`; (3) the distinct UAPI `HZ=100` and configuration-selected
internal `HZ`; and (4) consumer-context conversions of these replacement
tokens.  It must do so without a new shared module index, fabricated compile
time facade, or altered UAPI namespace.  This is a scope/ABI mapping blocker,
not permission to hard-code a target result.

No compiler, formatter, linker, test, rust-analyzer diagnostic, runtime, or
other build tool was invoked during this application.
