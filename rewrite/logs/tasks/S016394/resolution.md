# Applier resolution — S016394 / P01 / attempt 1

Applier: `applier` (`gpt-5.6-terra`, high)

The sealed candidate, the complete pinned source header, the direct pinned
SunRPC wrapper and consumer contexts, both independent reports and their
semantic-closure attestations, and the frozen S016394 records were reopened.
No compiler, formatter, linker, test, runtime tool, analyzer diagnostic,
historical Lupos source, or source edit was used.

## Dispositions

### P1 — accepted; unresolved and blocking

Pinned `include/uapi/linux/sunrpc/debug.h:10-28` defines the UAPI include guard
and thirteen object-like `RPCDBG_*` macros.  Its direct wrapper,
`include/linux/sunrpc/debug.h:25`, consumes that macro namespace using
`RPCDBG_##fac`; selected SunRPC sources, including `net/sunrpc/clnt.c:47-49`
and `net/sunrpc/svc.c:35-37`, define `RPCDBG_FACILITY` to the resulting macro
tokens.  The sealed candidate contains Rust constants only.  They cannot be
the C preprocessor tokens consumed by the pinned wrapper, and the frozen
S016394 symbol closure leaves the guard and every operative macro
`PENDING_REVIEW` on both targets.  No pinned source or frozen contract defines
an equivalent Rust/C UAPI bridge or permits dropping this interface.

### P2 — accepted; unresolved and blocking

Pinned `include/uapi/linux/sunrpc/debug.h:38-47` declares an anonymous C enum
whose enumerators have the values 1 through 8.  The values alone do not close
the selected declaration's target ABI and header-boundary contract.  The
frozen ABI rows for `anonymous_enum@38` retain layout, alignment, and export
kind as `PENDING_REVIEW` for both x86_64 and AArch64; the corresponding
lifetime rows retain ownership, lifetime, and synchronization context as
`PENDING_REVIEW`.  The direct pinned source search establishes no C/Rust
boundary or consumer contract that would prove the sealed constants are a
complete representation of the declaration.

### RUST-1 — accepted; same unresolved preprocessor boundary as P1

The candidate has no representation of `_UAPI_LINUX_SUNRPC_DEBUG_H_` or the
selected object-like macro namespace.  Rust module loading cannot substitute
for the C guard or for the token-pasting expansion proved above.  The fixed
`i32` items also do not establish the C consumer-context conversion contract.
This is source-level unresolved behavior, not a syntax issue.

### RUST-2 — accepted; same unresolved anonymous-enum closure as P2

There are no pointer, ownership-transfer, callback, atomic, or unsafe paths in
this small header candidate.  That does not establish the anonymous enum's
cross-language layout, alignment, export kind, or header-boundary semantics.
The pending records cannot be completed from the pinned source examined here.

## Result

No source edit is justified: inventing a bridge, changing the sealed candidate,
or closing pending ABI/lifetime fields without a frozen source-derived contract
would violate the zero-difference requirement.  S016394 must be `BLOCKED`
pending an explicit, source-proven UAPI preprocessor and anonymous-enum
boundary contract for both selected architectures.
