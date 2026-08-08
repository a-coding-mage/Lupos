# Parity review — S016315, slot 1

Scope reviewed: pinned `include/uapi/linux/nfsacl.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the current candidate
`src/include/uapi/linux/nfsacl.rs`, its current `candidate.diff`, the sealed
attempt-4 semantic proposal, frozen task rows, and direct local NFS consumers.
This was source inspection only; no compiler, formatter, test, or diagnostic
tool was invoked.

## Findings

### PARITY-1 — numeric UAPI macros have a different type contract

Linux symbols: `NFS_ACL_PROGRAM`, `ACLPROC2_NULL`, `ACLPROC2_GETACL`,
`ACLPROC2_SETACL`, `ACLPROC2_GETATTR`, `ACLPROC2_ACCESS`, `ACLPROC3_NULL`,
`ACLPROC3_GETACL`, `ACLPROC3_SETACL`, `NFS_ACL`, `NFS_ACLCNT`, `NFS_DFACL`,
`NFS_DFACLCNT`, `NFS_ACL_MASK`, and `NFS_ACL_DEFAULT`.

The pinned UAPI header defines each as an unsuffixed C integer token (lines
10, 12–20, 24–28, and 31); it does not declare any one of them as `u32`.
The candidate fixes every one to `pub const ...: u32`.  This changes the
macro's context-dependent C integer type/promotion contract into a fixed Rust
type.  The direct consumers demonstrate materially different contexts: the
procedure values are array designators in `fs/nfsd/nfs2acl.c`, the program
number initializes `.number` in `fs/nfs/nfs3client.c`, and the flag values are
used in mask expressions and passed through XDR-facing paths in
`fs/nfs/nfs3xdr.c` and `fs/nfsd/nfs3acl.c`.  No pinned local evidence supplied
with this task establishes `u32` as the exact Rust representation for every
such context on both frozen architectures.

The numeric values themselves match: `100227`; v2 procedures `0..4`; v3
procedures `0..2`; flags `0x0001`, `0x0002`, `0x0004`, `0x0008`, `0x000f`, and
`0x1000`.  The fixed `u32` type remains an unresolved mechanism/type change,
not proof of an exact macro translation.

### PARITY-2 — the UAPI include guard was changed into an exported value

Linux symbol: `_UAPI__LINUX_NFSACL_H`.

In the pinned header, lines 7–8 and 33 form a C preprocessor include guard:
the macro has an empty replacement list and controls repeated textual
inclusion.  The candidate instead exports
`pub const _UAPI__LINUX_NFSACL_H: () = ();`.  That introduces a public,
value-level Rust symbol with unit type; it neither has the C macro's empty
replacement semantics nor its preprocessing role.  The candidate's comment
asserting that a Rust module boundary supplies once-only behavior does not
preserve the original UAPI namespace/guard contract.  A source-established
mapping is required before this guard and its two selected conditional branches
can be marked `COMPLETE`.

### PARITY-3 — `candidate.diff` is not a faithful snapshot of the candidate

Linux notice: `(C) 2003 Andreas Gruenbacher <agruen@suse.de>` and the `File:
linux/nfsacl.h` notice from pinned header lines 2–5.

The current candidate retains both notices, and retains the exact SPDX
identifier `GPL-2.0 WITH Linux-syscall-note`; this portion of the source is
correct.  However, the supplied creation diff omits the `File:` and Andreas
Gruenbacher notices as well as explanatory candidate lines that are present in
the actual candidate.  Its `candidate_sha256` is consequently a hash of a
different source representation, while the semantic proposal binds that hash.
The candidate diff cannot serve as the required auditable candidate snapshot;
regenerate it from the actual candidate before relying on it for final review
or semantic closure.

## Result

FINDINGS.  No unauthorized Lupos branding, no function/static/layout change,
and no missing numeric value was found beyond the issues above.  The candidate
does retain the upstream SPDX identifier and Andreas Gruenbacher copyright
notice in the source file itself.
