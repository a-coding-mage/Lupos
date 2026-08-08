# S000200 resolution — attempt 1

Applier scope: `arch/arm64/include/asm/vncr_mapping.h` to
`src/arch/arm64/include/asm/vncr_mapping.rs`, pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

Source inspection only.  No compiler, formatter, linker, test, runtime, or
historical-source operation was used.

## RUST-S000200-01 — ACCEPTED; controlled requeue required

The candidate's `usize` type on every `VNCR_*` constant is not supported by
the pinned source.  `vncr_mapping.h:10-113` defines 104 object-like macros
whose replacements are unsuffixed hexadecimal integer literals.  The largest
is `0xB20`, so each is representable in the frozen target's signed 32-bit C
`int`; the frozen arm64 UAPI type source independently records `__s32` as
`__signed__ int` in `include/uapi/asm-generic/int-ll64.h:20-21`.  Nothing in
the header makes these literals pointer-sized or unsigned.

The direct pinned consumer confirms the required expression context.
`arch/arm64/include/asm/kvm_host.h:448-451` token-pastes each named macro into
`__VNCR_START__ + ((VNCR_ ## r) / 8)` while declaring `enum vcpu_sysreg`
(`:453` onward); the resulting values are bounded signed integer enum/index
arithmetic.  It performs no pointer arithmetic on a VNCR macro.  The header
itself likewise has neither storage nor a linkable ABI symbol.  Therefore a
Rust `usize` constant changes the source expression type, signedness, and
width without an upstream basis.  An explicit `i32` constant is the
source-faithful representation for every macro in this task.  Any later
translation needing a pointer displacement must make its conversion at that
specific source use site, after establishing that use site's bounds and
provenance; this constant-only header does not authorize a global
pointer-sized representation.

Required correction, limited to this frozen destination and with no value or
identifier changes:

1. Change all 104 declarations in
   `src/arch/arm64/include/asm/vncr_mapping.rs` from
   `pub const VNCR_*: usize = <same literal>;` to
   `pub const VNCR_*: i32 = <same literal>;`.
2. Regenerate the implementation evidence, candidate snapshot, and semantic
   closure candidate bindings for the corrected candidate.
3. Obtain fresh independent parity and Rust source reviews of that corrected
   candidate before a subsequent apply attempt.

This applier assignment is expressly limited to adjudication and resolution;
it does not modify the candidate, `candidate.diff`, or `implementation.md`.
Because the required type correction changes the candidate, the present
candidate hash and the present implementation, parity-review, Rust-review,
and semantic-closure attestations cannot seal a `DONE` transition.  The
appropriate terminal recommendation is **controlled requeue**, not `DONE` and
not `BLOCKED`: the pinned source supplies the exact correction and no
unresolved source/ABI/lifetime question remains for this finding.
