# Parity review — S016315

Reviewed `vendor/linux/include/uapi/linux/nfsacl.h` at Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/uapi/linux/nfsacl.rs` for the frozen `x86_64` and `aarch64`
configuration union.

## Finding P1 — upstream copyright notice omitted (required correction)

The candidate retains the exact SPDX identifier, but omits the upstream notice
`(C) 2003 Andreas Gruenbacher <agruen@suse.de>` from source lines 2–6. The
rewrite rules require retaining relevant upstream copyright notices. Restore
that notice as a Rust comment without changing the public UAPI constants.

Evidence: `vendor/linux/include/uapi/linux/nfsacl.h:1-6`; candidate
`src/include/uapi/linux/nfsacl.rs:1-5`.

## Verified parity

- All 15 selected public macro names and values are present as public `i32`
  constants: `NFS_ACL_PROGRAM=100227`; `ACLPROC2_{NULL,GETACL,SETACL,GETATTR,ACCESS}`
  = `0,1,2,3,4`; `ACLPROC3_{NULL,GETACL,SETACL}` = `0,1,2`; and
  `NFS_ACL=0x0001`, `NFS_ACLCNT=0x0002`, `NFS_DFACL=0x0004`,
  `NFS_DFACLCNT=0x0008`, `NFS_ACL_MASK=0x000f`,
  `NFS_ACL_DEFAULT=0x1000`. These are C `int`-representable unsuffixed
  integer constants on both frozen 64-bit architectures; `i32` preserves
  their source integer type and values.
- Both architecture inventories select the same include guard and 15 macros;
  there are no architecture or configuration-dependent value branches. Rust
  module inclusion supplies the one-definition protection corresponding to
  C's `_UAPI__LINUX_NFSACL_H` textual-include guard; no public guard symbol is
  part of the UAPI.
- The SPDX expression, Linux source path, exact frozen revision, task ID, and
  common-architecture provenance are present and correct. The constants are
  public, retain their UAPI names, and introduce no branding changes.
- Pinned consumers use these definitions as RPC program/procedure identifiers
  and ACL mask bits (for example `fs/nfs/nfs3client.c`, `fs/nfs/nfs3acl.c`,
  `fs/nfsd/nfs2acl.c`, and `fs/nfsd/nfs3acl.c`); the reviewed values preserve
  those protocol-visible identifiers and masks.

No compiler, formatter, linker, test, runtime, or diagnostic tooling was
invoked.
