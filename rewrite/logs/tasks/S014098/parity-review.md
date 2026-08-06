# Parity review — S014098 (slot 1)

Reviewer: parity reviewer (`gpt-5.6-terra`, high)

## Scope and preconditions

- Branch verified as `feat/bun-like-rewrite-test`.
- Queue row verified as `S014098`, `REVIEWING`, pipeline `P02`, mapping
  `include/linux/ioam6_genl.h` to `src/include/linux/ioam6_genl.rs`, with
  `common` architecture membership.
- Frozen Linux revision in both provenance and `vendor/linux.SHA` is
  `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Sole declared dependency `S016196` is `DONE` and maps the included UAPI
  header to `src/include/uapi/linux/ioam6_genl.rs`.

## Source comparison

The complete pinned Linux header has no declarations, storage, functions,
conditional configuration branches, or behavior of its own.  Its entire
operative content is the include guard and one include of
`uapi/linux/ioam6_genl.h`.

The candidate contains the required immutable provenance and exactly one
operative item:

```rust
pub use crate::include::uapi::linux::ioam6_genl::*;
```

That public glob re-export is the Rust counterpart of the Linux header's sole
UAPI include: it exposes the complete dependent IOAM6 generic-netlink UAPI
surface at this kernel-header path without adding any state, symbols, runtime
logic, configuration behavior, synchronization, or alternate ABI.

I also compared the re-export target with the full pinned
`include/uapi/linux/ioam6_genl.h`.  The target supplies every included UAPI
item: the two generic-netlink name/version macros; all namespace, schema, and
padding attribute constants and their maxima; all command constants and their
maximum; the event-group name; both event enum tags and every event/event-
attribute enumerator and maximum; and the schema-data length expression.  No
kernel-only declaration from the wrapper header is omitted, because none
exists upstream.  The frozen scope/header-closure evidence selects this header
for both x86_64 and aarch64 through `net/ipv6/af_inet6.o`; no architecture- or
Kconfig-specific branch exists in the pinned header.

No branding delta is present.

## Findings

None.  The source-level wrapper faithfully preserves the complete selected
Linux header semantics and its dependency surface.  The applier must still
close the task's manifest `PENDING_REVIEW` guard/semantic-record entries as
required by the workflow before a `DONE` transition; this is not a candidate
source discrepancy.

## Verdict

ACCEPT — no parity-source change requested.
