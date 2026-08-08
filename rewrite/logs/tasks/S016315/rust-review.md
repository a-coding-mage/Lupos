# Rust source review — S016315, attempt 4, slot 2

Result: **FINDINGS**. I used only the current candidate, the pinned Linux source
at `425f94c2954b1fe80ebdbf9b29854e89750355df`, the current task proposal, and
the frozen task records. No compiler, formatter, test, rust-analyzer diagnostic,
historical Rust source, or other review artifact was used.

## RUST-001 — Fixed `u32` constants lose C `int`/promotion semantics

Candidate lines 16-36 declare every translated macro as `u32`. Every replacement
literal in pinned `include/uapi/linux/nfsacl.h:10-31` is unsuffixed and fits C
`int`, so it has `int` type before C's usual arithmetic conversions. This is
context-dependent: `vendor/linux/fs/nfs_common/nfsacl.c:93-94,154-156` declares
`typeflag` as `int`; `vendor/linux/fs/nfs/nfs3xdr.c:1360-1363` and
`vendor/linux/fs/nfsd/nfs3acl.c:188-190` pass `NFS_ACL_DEFAULT` to it. Conversely,
`vendor/linux/fs/nfs_common/nfsacl.c:238` combines a `u32` `ntohl` result with
`~NFS_ACL_DEFAULT`, where C converts the signed macro for that operator.

Fixing every name as `u32` changes the public Rust type and erases the required
contextual conversion for signed `int` parameters and signed complement/mask
expressions. Non-negative values do not make the type and operator rules equal.
The applier must preserve the literal/promotion contract at each translated
consumer, including both `int typeflag` and `u32` mask paths.

## RUST-002 — The include guard is a new public Rust API item

The empty C preprocessor guard `_UAPI__LINUX_NFSACL_H` at pinned header lines
7-8 and 33 suppresses repeated textual inclusion only. Candidate lines 11-14
turn it into `pub const _UAPI__LINUX_NFSACL_H: () = ();`. A Rust module boundary
already supplies module inclusion semantics; this public zero-sized typed item
does not control conditional inclusion or represent a C value/ABI entity.
`#[doc(hidden)]` does not make it private. The candidate therefore adds a public
Rust API name and type absent from the UAPI contract. Remove it or provide
pinned-source ABI evidence justifying the addition.

## Other Rust-semantics audit

This constant-only candidate contains no unsafe block/function, pointers or
references, allocation, interior mutability, Drop/pinning/Send/Sync behavior,
callbacks/RCU/refcounts, FFI or `repr(C)` layout, bounds access, panic path, or
Rust test configuration. No independent finding arose in those categories.
The frozen ABI.tsv and LIFETIMES.tsv have no rows for this header, so they do
not justify either substitution above.

The slot-2 semantic attestation maps RUST-001 to all non-guard operative-macro
status keys for both architectures and RUST-002 to both guard status keys.
