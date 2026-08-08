# Parity review — S016394 / P01 / attempt 1

Scope reviewed independently: `vendor/linux/include/uapi/linux/sunrpc/debug.h`,
the frozen S016394 scope/symbol/ABI/lifetime records, the candidate snapshot,
and direct pinned consumers.  No compiler, formatter, test, analyzer, or
historical Lupos source was used.

## Findings

### P1 — `RPCDBG_*` macro interface and include guard are not translated

`debug.h:10-28` exports an include guard plus thirteen object-like C macros.
The candidate turns every `RPCDBG_*` replacement token into a typed Rust
`i32` item and drops `_UAPI_LINUX_SUNRPC_DEBUG_H_` entirely.  This is not an
equivalent interface for the selected C/preprocessor consumers.  In particular,
the pinned `include/linux/sunrpc/debug.h:25` forms `RPCDBG_##fac` and then
uses the resulting macro in `rpc_debug & RPCDBG_##fac` (lines 25-28), while
multiple selected SunRPC C units define `RPCDBG_FACILITY` to these macros (for
example `net/sunrpc/clnt.c:47-49` and `net/sunrpc/svc.c:35-37`).  Rust constants
are neither preprocessing replacement tokens nor usable by the token-pasting
mechanism.  They also force a signed `i32` operand where the cited C expression
uses the unsuffixed `int` macro under C's usual arithmetic conversions against
an `unsigned int` `rpc_debug`; no frozen consumer-boundary mapping establishes
an exact Rust representation of that conversion or of the public guard.

This affects both architectures' `SYMBOLS.tsv` macro/guard closure rows.  The
proposal's `SOURCE_REVIEWED_VALUE`/`COMPLETE` assertions do not supply the
missing preprocessor and integer-conversion mapping.

### P2 — anonymous-enum ABI/lifetime closure is asserted without source proof

`debug.h:38-47` is an anonymous enum declaration.  The candidate emits eight
`i32` constants and a comment asserting that the C enum has type `int`, but it
does not preserve or establish the declaration's C-facing type/interface.
Both frozen `ABI.tsv` rows for `anonymous_enum@38` leave layout, alignment, and
export-kind `PENDING_REVIEW`; both `LIFETIMES.tsv` rows leave ownership,
lifetime, and synchronization `PENDING_REVIEW`.  The source line contains no
target ABI/layout evidence, and the candidate's comment is not such evidence.
The semantic proposal changes each of those records to
`SOURCE_REVIEWED_VALUE`/`COMPLETE` without a source-derived value.  Exact
cross-language enum treatment therefore remains unresolved for x86_64 and
AArch64.

## Conclusion

Reject.  The literal numeric values are present, but the selected header's
preprocessor, C-integer-context, and pending anonymous-enum contract are not
preserved or closed from pinned-source evidence.  This needs an explicit,
source-proven C/Rust header boundary before the task can be accepted.
