# Rust source review — S013591

Review scope: pinned `vendor/linux/include/linux/circ_buf.h`, current
`src/include/linux/circ_buf.rs`, the frozen task record, and necessary pinned
call-site context. This was manual source inspection only; no compiler,
formatter, linker, test, rust-analyzer diagnostic, or runtime tool was used.

## Finding RUST-1 — `*_TO_END` macro evaluation count is changed (high)

The C definitions deliberately evaluate `head` and `tail` once, but they do
not evaluate `size` once: `CIRC_CNT_TO_END` uses `(size)` in both the `end`
initializer and `((size)-1)` mask (pinned header lines 26–29), and
`CIRC_SPACE_TO_END` likewise uses `(size)` in both initializers (lines 32–35).
Consequently a C argument expression supplied for `size` is evaluated twice,
with the second evaluation occurring after the first local has been initialized.

The Rust `CIRC_CNT_TO_END!` macro binds `$size` once to `__circ_size` and
uses that local for both calculations (candidate lines 65–70). The Rust
`CIRC_SPACE_TO_END!` macro does the same (lines 79 onward). This silently removes
the second volatile/accessor/side-effecting evaluation and changes its ordering
relative to the `head`/`tail` evaluations. The pinned C comment promises only
single evaluation of `head` and `tail`; it does not authorize caching `size`.

This is a mechanism and observable-side-effect change in a public macro. The
translation must preserve the two `size` evaluations and their source-order
positions (while still ensuring `head` and `tail` are each evaluated once), or
the task must be blocked if Rust macro semantics cannot express that exact
contract for the frozen callers.

## Other Rust-semantics checks

`circ_buf` has `#[repr(C)]`, uses a raw mutable byte pointer and C `int`-width
indices, and has no `unsafe` block, `Drop`, allocation, callback, pinning, or
interior-mutability operation to audit in this file. The raw pointer avoids
creating Rust references with stronger aliasing or lifetime guarantees. No
additional ownership, layout, provenance, Send/Sync, cast, panic, or unsafe
finding was established from the permitted source inputs.
