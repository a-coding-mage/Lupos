# Rust source review — S015666 / attempt 1

Reviewed `src/include/net/tcp_states.rs` against the pinned
`vendor/linux/include/net/tcp_states.h` and direct pinned consumers.  This was
manual source inspection only; no compiler, formatter, analyzer, test, or
historical Lupos source was used.

## Findings

### RUST-1 — fixed `i32` constants do not preserve C expression context

The C anonymous-enum constants and `TCP_STATE_MASK` / `TCP_ACTION_FIN` macros
are integer constant expressions.  Their operands undergo the C usual
arithmetic conversions at each consumer.  The candidate turns every value into
a Rust `pub const ...: i32`; that fixes the type before the consumer has an
opportunity to select the corresponding signed or unsigned operation.

This is a material mismatch, not merely a type-style difference.  For example,
`vendor/linux/net/ipv4/tcp_diag.c:317-319` stores `r->idiag_states` in `u32`
and applies `TCPF_SYN_RECV` / `TCPF_NEW_SYN_RECV`; C converts the positive enum
constant for the `u32` bitwise operation.  `vendor/linux/net/ipv4/tcp.c:3037-3061`
also uses the state and action expressions in designated `unsigned char` array
initializers and subsequently as `int` masks.  A monomorphic Rust `i32` public
constant has no source-proven equivalent conversion behavior in those contexts.
The frozen records leave the macro selection expressions and the enum constant
mechanical values pending, and the candidate does not supply a context-preserving
mapping.

### RUST-2 — anonymous C enum / ABI records are closed without an established mapping

The two source anonymous enums are declared with C enum semantics.  Although
their listed values fit in `int`, the frozen ABI records for both architectures
still mark layout, alignment, and export treatment `PENDING_REVIEW`.  Replacing
them with unrelated Rust `i32` constants supplies neither a source-established
anonymous-enum representation nor a C-facing ABI boundary.  The direct consumer
set includes socket state fields, netlink `u32` masks, C designated initializers,
and trace/event macro expansion.  No local source evidence establishes that
exporting these fixed Rust constants is the required replacement for all such
uses.

### RUST-3 — include-guard macro has no mapped Rust mechanism

`_LINUX_TCP_STATES_H` is selected as an operative macro for both frozen
architectures.  The candidate omits it and does not record the Rust module/import
mechanism that replaces the C preprocessor guard.  Because headers are included
through macro-expansion contexts, the source record cannot be closed as
`COMPLETE` without a source-proven mapping of the guard's repeated-include and
visibility semantics.

## Scope of review

This header owns no pointers, allocation, callbacks, locking, `Drop`, or
`unsafe` blocks, so those categories introduce no additional file-local finding.
The rejection is on integer-expression semantics and unresolved ABI/macro
closure, not on a compiler diagnostic.
