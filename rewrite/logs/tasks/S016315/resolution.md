# Applier resolution — S016315, attempt 4

Scope reopened: the complete pinned `include/uapi/linux/nfsacl.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, current sealed candidate,
candidate snapshot, both independent reports and their semantic-closure
attestations, `SCOPE.tsv`, `SYMBOLS.tsv`, `ABI.tsv`, `LIFETIMES.tsv`, and
direct pinned NFS consumers.  This was manual source inspection only; no
compiler, formatter, linker, test, diagnostic, or historical Rust source was
used.

## Dispositions

### PARITY-1 — accepted; unresolved, task must block

Pinned `include/uapi/linux/nfsacl.h:10-31` defines each value as an unsuffixed
C integer literal.  Each fits `int`; the source does not declare a `u32` type.
The direct contexts are materially distinct: `fs/nfs_common/nfsacl.c:93-94`
and `154-156` use `int typeflag`, while `fs/nfs_common/nfsacl.c:238` evaluates
`ntohl(*p++) & ~NFS_ACL_DEFAULT` with a `u32` left operand.  In addition,
`fs/nfs/nfs3xdr.c:1360-1363` passes `NFS_ACL_DEFAULT` to the `int typeflag`
parameter.  The sealed candidate fixes every replacement as `pub const ...:
u32`, which cannot preserve both C contexts and their conversions/promotions.

Neither the frozen `ABI.tsv` nor `LIFETIMES.tsv` has a row for this header, and
the frozen `PORTING.md` supplies no Rust representation/consumer bridge for
context-dependent C object-like integer macros.  Replacing `u32` with another
fixed Rust type, or adding a new macro bridge, would be a new unreviewed
design.  No exact source-proven correction exists within this sealed task.

### PARITY-2 — accepted; unresolved, task must block

Pinned `include/uapi/linux/nfsacl.h:7-8,33` gives
`_UAPI__LINUX_NFSACL_H` an empty replacement list solely to implement C
conditional textual inclusion.  The candidate adds a public typed Rust item,
`pub const _UAPI__LINUX_NFSACL_H: () = ();`, which is neither an empty macro
replacement nor a preprocessor guard.  The source header and frozen ABI
records provide no evidence for exporting that Rust value, and the frozen
records provide no mapping for the two selected guard conditionals.  Removing
it or defining an alternative guard bridge would change the sealed candidate
and require a new, source-proven design and two fresh reviews; it cannot be
accepted here.

### PARITY-3 — accepted; unresolved, task must block

The actual candidate retains the upstream `File: linux/nfsacl.h` and Andreas
Gruenbacher notices, but `candidate.diff` omits them and other candidate lines.
The proposal records `candidate_sha256`
`5c9040c06d982b1f7ffe3e2344d54386170871dc436ad134da19ad81e22e3a3c`, the
hash of `candidate.diff`; the current candidate instead hashes to
`7cccbfc244ac3372e1eb868f14cd75dde504c104dacfc5fed113868d7110247c`.
Consequently the sealed proposal/reviews do not bind an auditable snapshot of
the actual candidate.  Regenerating the snapshot would alter sealed evidence
and require fresh independent reviews, so it cannot support `DONE`.

### RUST-001 — accepted; unresolved, task must block

This is the Rust-semantics aspect of PARITY-1.  `u32` erases the source
literal's `int` type before C usual arithmetic conversions.  The pinned
`int typeflag` calls and `u32 & ~NFS_ACL_DEFAULT` path above establish that a
single fixed public Rust type is not a complete mapping.  No frozen Rust bridge
establishes how each translated consumer preserves the context-sensitive C
conversion behavior.

### RUST-002 — accepted; unresolved, task must block

This is the Rust-semantics aspect of PARITY-2.  `#[doc(hidden)]` does not make
the public unit constant private or restore the C preprocessor role.  No
source-backed ABI or module-boundary mapping authorizes this additional public
value-level API.

## Final disposition

No source file was changed: the candidate is sealed, and the exact corrections
needed for the C macro and include-guard contracts are not established by the
frozen Rust records.  The task is therefore `BLOCKED`, with all associated
semantic records remaining unresolved; no `DONE` claim is made.
