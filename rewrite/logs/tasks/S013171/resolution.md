# S013171 applier resolution — attempt 1

## Scope and method

Applier review reopened the complete pinned
`vendor/linux/include/dt-bindings/leds/common.h` at Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the current leased candidate,
both review reports, the S013171 frozen scope/file-map/queue rows, selected
symbol rows, and the task's semantic-closure proposal.  The task remains the
frozen `RUST_TRANSLATE` mapping
`include/dt-bindings/leds/common.h` ->
`src/include/dt-bindings/leds/common.rs` for `common` architectures.

No compiler, formatter, linker, rust-analyzer diagnostic, test, runtime, or
toolchain command was invoked.  No candidate source, review report, manifest,
or queue row was changed by this adjudication.

## Finding dispositions

### F1 — `__DT_BINDINGS_LEDS_H` include guard

**Disposition: ACCEPTED — BLOCKING.**

Pinned source lines 12–13 perform the C preprocessing conditional and define
the guard macro; line 114 closes that conditional.  The frozen `SYMBOLS.tsv`
rows select `ifndef@12`, `__DT_BINDINGS_LEDS_H`, and `endif@114` for both
`aarch64` and `x86_64`.  The candidate contains neither a preprocessing guard
nor the selected macro.  The task semantic-closure proposal's `COMPLETE`
claim for those records is therefore not established by the candidate.

A Rust module's single-definition loading cannot reproduce C preprocessing
state, repeated textual inclusion, or subsequent `#ifdef
__DT_BINDINGS_LEDS_H` observation.  Adding a Rust item with that spelling
would not supply those semantics.  No faithful representation for this
selected C preprocessor contract is established by the frozen source and the
current one-file Rust mapping; inventing one would be an unreviewed design.

### F2 — 21 unsuffixed integer macros represented as `u32`

**Disposition: ACCEPTED — BLOCKING.**

Pinned source lines 16–41 define the listed binding names as object-like,
unsuffixed decimal C integer-literal macros.  Their expansion is an `int`
expression on the frozen targets and takes part in the receiving C expression's
usual promotions and conversions.  The candidate instead supplies typed
`u32` constants.  Equal non-negative values do not preserve signedness,
promotion, comparison, unary operation, or contextual conversion behavior.

Changing `u32` to `i32` would still create a Rust item rather than a C macro
expansion and would not recreate the selected macro's C contextual semantics.
The pinned file has no consumer-specific Rust contract from which a narrower
representation could be proven exact.  Thus a source-only correction within
this frozen mapping cannot be accepted.

### F3 — 49 C string-literal macros represented as `&str`

**Disposition: ACCEPTED — BLOCKING.**

Pinned source lines 46–112 define each listed name as an object-like C string
literal macro.  A C string literal is an array initialized with an appended
NUL byte and has C array-to-pointer and `sizeof` behavior at each expansion.
The candidate's `&str` values have a slice representation, UTF-8 invariant,
and no C-string terminating-NUL or C array expression contract.  The visible
characters alone are insufficient evidence of parity.

Replacing these with NUL-terminated byte arrays would still be typed Rust
items rather than C array-literal macro expansions, and no frozen source
evidence establishes the necessary Rust consumer interface.  Such a carrier
would be a new unreviewed design, so this finding cannot be corrected
faithfully in the present task.

### F4 — SPDX identifier

**Disposition: ACCEPTED — NON-BLOCKING IN ISOLATION.**

Pinned source line 1 is `SPDX-License-Identifier: (GPL-2.0 OR BSD-2-Clause)`.
The candidate instead states `GPL-2.0-only`.  This does not retain the
upstream identifier or its BSD-2-Clause alternative.  It could be corrected
only by changing the candidate and then regenerating the candidate evidence
and repeating both independent reviews; that correction alone cannot resolve
F1–F3.

## Terminal recommendation

**Recommend `BLOCKED`, not `DONE` and not a controlled requeue.**  The three
accepted blocking findings arise from selected C preprocessor and macro
semantics for which the frozen one-to-one Rust task provides no faithful,
source-established representation.  A requeue that merely changes literal
types, adds NUL bytes, or adds a Rust guard-named item would weaken the Linux
contract.  A future path requires a reviewed scope/translation-design decision
that establishes an exact treatment for selected C header preprocessor and
macro contracts; it is outside this applier's frozen file scope.

The task must not transition to `DONE`: the candidate remains unchanged but
does not resolve F1–F4, and its semantic-closure proposal contains unresolved
claims for the selected records.  Per coordinator direction, this applier has
made no queue mutation.
