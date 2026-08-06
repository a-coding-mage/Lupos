# Parity review — S013801 (attempt 2)

Reviewer: parity reviewer (`gpt-5.6-terra`, high)

Scope reviewed: `src/include/linux/dqblk_v1.rs` against pinned
`vendor/linux/include/linux/dqblk_v1.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, for the frozen common/x86_64/aarch64
task context.

Result: no parity findings.

The candidate preserves every selected operative macro as a public Rust
constant with the same identifier and value: `V1_INIT_ALLOC = 1`,
`V1_INIT_REWRITE = 1`, `V1_DEL_ALLOC = 0`, and `V1_DEL_REWRITE = 2`.
In the original header each replacement list is an unsuffixed decimal integer
literal, whose C type is `int`; the candidate's `core::ffi::c_int` preserves
that integer type at the Rust/C boundary on both frozen architectures. The
macros have no operands, evaluation, side effects, precedence behavior,
storage, linkage, layout, or pointer provenance requirements beyond those
values and their C `int` type. The header has no includes, declarations,
conditionals selected by configuration, or ABI-bearing structures/functions
that require an additional translation.

The include guard is correctly not represented as a runtime or ABI artifact;
Rust module inclusion supplies its corresponding single-definition role.

No source was modified. No compiler, formatter, rust-analyzer, build, test,
debugger, or runtime tool was used.
