# Rust review — S015110

Reviewed `src/include/linux/sunrpc/xprtrdma.rs` against pinned
`vendor/linux/include/linux/sunrpc/xprtrdma.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the selected direct consumers in
`vendor/linux/net/sunrpc/xprtrdma/transport.c` and
`vendor/linux/net/sunrpc/xprtrdma/xprt_rdma.h`, and the selected x86_64/AArch64
symbol, ABI, and lifetime records.

## Result

REJECT — one high-severity Rust value-semantics/ABI issue requires applier
resolution.

### RUST-1 — `rpcrdma_memreg` is not usable as the C integer-constant API (high)

The C declaration at `include/linux/sunrpc/xprtrdma.h:62-70` supplies an enum
tag plus enumerator *integer constant expressions*. The selected consumer at
`net/sunrpc/xprtrdma/transport.c:70` initializes an `unsigned int` from
`RPCRDMA_FRWR`; at lines 81-82 it initializes another `unsigned int` from
`RPCRDMA_BOUNCEBUFFERS` and evaluates `RPCRDMA_LAST - 1`. Those implicit
integer conversions and arithmetic are part of the header's public
kernel/user-space-number API.

The candidate replaces the enumerators with variants of a nominal Rust
`#[repr(C)] enum`. A variant is not an integer constant expression usable with
the C operations above: it cannot be implicitly assigned to the translated
unsigned-integer destinations and `RPCRDMA_LAST - 1` has no subtraction
operation without a caller-specific cast. Such casts would also turn the
header's ordinary numeric API into a nominal valid-enum domain, even though the
Linux tunable values are stored and obtained as integers.

`#[repr(C)]` does not repair those Rust value semantics, and the selected
`ABI.tsv` rows for this enum remain `PENDING_REVIEW` on both architectures.
The applier must establish the frozen compiler/target representation and map
the tag and all eight names to an explicitly sized integer-compatible form
that preserves integer assignment, arithmetic, and values outside the named
set where Linux permits them. Do not introduce a Rust enum validity invariant
at this API boundary.

## Other checks

- All six macro values and the ordered numeric sequence 0 through 7 match the
  pinned header.
- The header declares no storage-owning object, pointer, reference, atomic,
  lock, refcount, callback, or `unsafe` operation. No ownership, aliasing,
  `Send`/`Sync`, pinning, or drop-time concern is introduced apart from the
  erroneous nominal-enum domain above.
- `RPCRDMA_MAX_INLINE` remains a signed `int`-typed C integer constant in the
  source but is only used by the selected header consumer as a non-negative
  compile-time arithmetic value; no separate Rust provenance issue was found.

No source files were edited by this reviewer, and no build, formatter, test,
or runtime command was run.
