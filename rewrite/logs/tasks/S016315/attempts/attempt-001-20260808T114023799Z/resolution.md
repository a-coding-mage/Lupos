# Application resolution — S016315, attempt 1

- Task/pipeline: `S016315` / `P01`
- Pinned Linux source: `include/uapi/linux/nfsacl.h`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Frozen bindings: Phase 0 identity `0123af9e96b58af4e98b91a2de5608d36b63c67864b9041d3340587f3dbe40d2`; queue fingerprint `cfa8dcf0d81c408de38d4202fe1ce9d9e5e301d1e67439c9530da94b1e64ee3f`.

## Independent application audit

The complete pinned header is 33 lines and contains an SPDX expression, the
Andreas Gruenbacher copyright notice, a preprocessor guard, and 15 numeric
UAPI macros.  The sealed candidate retains the SPDX expression, module
provenance, every macro spelling, and every literal as an `i32`; its source
hash is `6ec8c6bbe66527a98d7c9eb3949856bd042cae61c5ba8c24ffc7c86c1512036d`.
The sealed candidate diff hash is
`4ecfff2cd1276f1ce706d72297ea2d522857f1ebd402e3b712db55e0235a6766`.

Manual source inspection of the direct pinned contexts independently confirms
the mappings: `include/linux/nfsacl.h:13` includes this UAPI header and repeats
the same notice at lines 2--6; `include/linux/nfs_xdr.h:899--908,1024--1027`
uses the mask values in `int` fields; `fs/nfs/nfs3client.c:17--23` uses the
program literal; `fs/nfs/nfs3xdr.c:2488--2507` and
`fs/nfsd/nfs2acl.c:325--385` use the procedure literals as table indices and
procedure numbers; and `fs/nfs`, `fs/nfsd`, and `fs/nfs_common` use the flag
literals in mask and XDR paths.  There are no functions, storage objects,
layouts, ownership, locking, refcounting, RCU, or unsafe boundaries in this
header.  The C guard is preprocessor-only and has no Rust runtime/linkage
counterpart.  No compiler, formatter, linker, test, runtime, or diagnostic
tool was used.

## Finding dispositions

### P1 — upstream copyright notice omitted (parity slot 1)

**Disposition: ACCEPTED; correction required through controlled requeue.**

Pinned `vendor/linux/include/uapi/linux/nfsacl.h:2--6` contains
`(C) 2003 Andreas Gruenbacher <agruen@suse.de>`, and the direct wrapper
`vendor/linux/include/linux/nfsacl.h:2--6` confirms its relevance.  The sealed
candidate lacks that notice.  AGENTS.md requires retaining relevant upstream
copyright notices, so the omission cannot be accepted or treated as an
allowlisted branding difference.

The exact correction is established: a fresh candidate must retain the
upstream notice alongside its SPDX/provenance header, without changing any
macro mapping.  Applying it here would change the sealed candidate and its
candidate-diff/proposal/review bindings.  This applier therefore makes no
source edit and requests the queue tool's controlled requeue, which archives
attempt-1 evidence and resets the task for a newly reviewed candidate.

### R1 — slot-2 Rust review approval

**Disposition: NOT ACCEPTED AS REVIEW EVIDENCE; independently re-evaluated.**

The slot-2 reviewer disclosed cross-task task-log exposure after recording its
approval, so that approval is invalid for acceptance.  This application does
not rely on it.  Independent manual source inspection found no additional Rust
semantic defect in the fifteen literal mappings, but the corrected candidate
must receive a clean, independent slot-2 review after requeue.

## Outcome

`DONE` is not permitted for attempt 1: P1 remains uncorrected in the sealed
candidate and a valid independent slot-2 review must cover the corrected
candidate.  Exact source evidence is sufficient to specify the correction, so
this is a controlled requeue rather than `BLOCKED`.
