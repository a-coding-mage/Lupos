# S000805 resolution — attempt 1

## Outcome: BLOCKED

Both review findings are sustained.  No source-faithful, path-local Rust
representation for the two C initializer-list fragments was established from
the pinned source, so the candidate is not changed and this attempt must not
be sealed as `DONE`.

### P1 / RUST-S2-001 — `VMX_EXIT_REASONS` and `VMX_EXIT_REASON_FLAGS`

**Disposition: sustained; exact translation is blocked.**

`arch/x86/include/uapi/asm/vmx.h:101-166` defines `VMX_EXIT_REASONS` as 65
comma-separated brace initializers with no enclosing aggregate.  Lines 168-169
define `VMX_EXIT_REASON_FLAGS` in the same fragment form.  The fragments are
not standalone array objects: `arch/x86/kvm/trace.h:383-389` passes each one to
variadic trace macros, and the eventual definitions in
`include/trace/stages/stage3_trace_output.h:75-80` and `85-90` place the
received tokens inside caller-owned `static const ... symbols[] = { ... }` and
`__flags[] = { ... }` initializers.  The independent perf caller is explicit:
`tools/perf/util/kvm-stat-arch/kvm-stat-x86.c:12` passes
`VMX_EXIT_REASONS` to `define_exit_reasons_table`, whose definition at
`tools/perf/util/kvm-stat.h:129-132` inserts `symbols` before its own sentinel
`{ -1, NULL }` in a caller-owned initializer.

The candidate's `VMX_EXIT_REASONS!()` and `VMX_EXIT_REASON_FLAGS!()` instead
expand to complete Rust arrays.  That changes the required fragment category,
forces a tuple-array type, and cannot occupy the established C call sites'
aggregate-entry position without becoming a nested array.  Its exported,
unqualified constant tokens also require invocation-site bindings, unlike the
header expansion after C preprocessing.

No pinned source evidence establishes an equivalent Rust token-fragment form
that can be spliced into arbitrary caller-owned aggregate initializers while
preserving those caller-selected element types and the adjacent sentinel.
Replacing the fragments with an array, function, trait, iterator, or a
caller-specific representation would be an intentional semantic/interface
change.  Under the zero-difference contract, that uncertainty is a blocker.

The source file is deliberately left unchanged: correcting it would invalidate
the sealed candidate and both reviews, requiring controlled requeue and fresh
independent review rather than an application-stage reseal.

No compiler, formatter, linker, test, debugger, emulator, rust-analyzer
diagnostic, or runtime command was used.
