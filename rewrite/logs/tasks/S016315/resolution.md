# Resolution — S016315

Applier source review against `vendor/linux/include/uapi/linux/nfsacl.h` at
Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df` is complete.  No
compiler, formatter, linker, test, runtime, or diagnostic tooling was used.

## Parity finding P1 — resolved

Restored the upstream immutable notice `(C) 2003 Andreas Gruenbacher
<agruen@suse.de>` as a Rust comment immediately after the SPDX identifier.
The public UAPI identifiers and their values are unchanged.

## Rust finding 1 — resolved with the source-derived conversion boundary

The replacement lists in the pinned header are unsuffixed decimal or
hexadecimal integer literals (`100227`, `0` through `4`, `0` through `2`, and
`0x0001` through `0x1000`).  Each value is representable by C `int` on both
frozen 64-bit targets.  Consequently the header's direct macro expression
type is `int`; the candidate's public `i32` constants preserve that direct
source representation.

The reviewer correctly identified unsigned consumers, but that conversion is
not a property stored in the macro definition.  It happens at the C use site:
`struct nfsd3_getaclargs.mask` and `struct nfsd3_setaclargs.mask` are `__u32`
in `vendor/linux/fs/nfsd/xdr3.h:103-106`, and their expressions in
`vendor/linux/fs/nfsd/nfs3acl.c:43,49,61,151,153,156,186,189` apply C's usual
arithmetic conversions to the `int` macro operand.  The same header constants
also retain their direct signed use: `NFS_ACL_DEFAULT` is passed as the
`int typeflag` argument of `nfsacl_encode` and `nfs_stream_encode_acl`, whose
definitions declare that parameter as `int` in
`vendor/linux/fs/nfs_common/nfsacl.c:93-94,152-154`.

Rust has no object-like public constant whose integer type is contextually
converted as a C macro expansion is.  Changing the canonical header values to
`u32` would instead lose the direct `int` representation and require casts at
the signed `typeflag` consumers.  Therefore this header retains the exact
direct `i32` representation.  When the selected consumers are translated,
their `u32` mask operations must perform the explicit, value-preserving
conversion at that use boundary; signed `typeflag` uses take the `i32` value
directly.  All values are non-negative and at most `0x1000`, so either
source-derived conversion is exact.  This closes the task's pending semantic
question without changing Phase 0 manifests.

## Rust finding 2 — resolved

Resolved by the same restored upstream notice described for P1.

All fifteen selected public macro names, values, common architecture scope,
provenance, and the absence of a Rust ABI object/linkage remain verified from
the pinned source.  The C include guard is a preprocessing mechanism with no
public Rust UAPI item; Rust module inclusion provides its corresponding
one-definition behavior.
