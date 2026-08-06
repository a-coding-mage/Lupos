# Resolution — S016168

Applied after independently reopening the complete pinned source
`vendor/linux/include/uapi/linux/if_infiniband.h` at Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the candidate, frozen queue and
scope/symbol evidence, selected `net/ipv6/addrconf.c` consumer, and both
independent review reports.

## Finding disposition

| Finding | Disposition | Source evidence |
| --- | --- | --- |
| P1 — upstream dual-license notice materially truncated | Resolved. The candidate now retains verbatim the complete dual-license choice, both license locations, warranty/liability disclaimer, Topspin copyright notice, and `$Id$` notice from `if_infiniband.h:2-23`, after its required immutable provenance. | `vendor/linux/include/uapi/linux/if_infiniband.h:2-23` |

## Semantic closure

`INFINIBAND_ALEN` remains `pub const INFINIBAND_ALEN: i32 = 20`. The pinned
macro is an unconditional unsuffixed C integer constant (`int`) whose exact
value is 20; both frozen x86_64 and AArch64 targets have 32-bit `int`.
Accordingly, `i32` preserves the selected consumer comparison's value and
integer type. The C include guard is solely repeated-textual-inclusion
protection and has no Rust item, storage, linkage, layout, ownership, locking,
RCU, refcount, allocation, error, or runtime counterpart.

All task-local semantic records are closed: the selected common macro,
architecture membership, source provenance, UAPI licensing notice, and the
consumer-visible name/value/type are established from the pinned source and
frozen metadata. No unresolved ABI, lifetime, ownership, synchronization, or
dependency question remains for S016168.

This application used manual source inspection only; no compiler, formatter,
linker, test, runtime, or diagnostic tool was run or used.
