# Resolution — S016368 / P01 / attempt 3

Reviewed manually against the complete pinned
`vendor/linux/include/uapi/linux/securebits.h`, the current candidate and
candidate diff, both current review reports, the pinned wrapper
`vendor/linux/include/linux/securebits.h`, and the direct pinned-source
references to `issecure_mask` in `security/commoncap.c`.  No compiler,
formatter, linker, test, runtime tool, or historical Lupos source was used.

## P1 — `issecure_mask` all-input C `int` shift contract

**Disposition: accepted; unresolved — recommend `BLOCKED`.**

The pinned UAPI definition at `include/uapi/linux/securebits.h:9` is the
public function-like C macro `(1 << (X))`: its left operand is an unsuffixed C
`int`, while `X` remains a general macro argument.  The candidate instead
exports `($x:expr) => (1_i32 << ($x))` at
`src/include/uapi/linux/securebits.rs:14-18`.  Although both expressions
evaluate their argument once, this does not establish a parity-preserving Rust
contract for every C expression accepted by the public macro, including the
C integral-promotion and invalid-shift-count domain.

The complete pinned local reference search finds no invocation with a dynamic
or out-of-range count: the header's own derived masks use fixed indices 0
through 11, and `security/commoncap.c:994,1394,1396` uses
`SECURE_KEEP_CAPS` only.  `include/linux/securebits.h:7` defines the general
wrapper `issecure(X)`, but the examined direct uses remain fixed securebit
indices.  Those facts prove the named constants and selected in-tree calls,
not an argument-domain restriction for the public UAPI macro.

Neither the pinned header, its direct callers, nor the frozen task ABI/lifetime
records supplies the missing all-input C-to-Rust shift conversion/invalid-count
contract.  The Rust review's acceptance is valid for the established fixed
0--11 call domain, but it cannot disprove the parity finding for the public
parameterized macro.  Defining a narrower Rust-only domain or choosing a
Rust overflow/checked-shift behavior would be a new unreviewed design and
would weaken the public Linux behavior without upstream evidence.

No source edit is justified by the available evidence.  The task must remain
blocked until authoritative pinned local evidence establishes the required
public macro contract; it must not transition to `DONE` on the fixed-index
evidence alone.
