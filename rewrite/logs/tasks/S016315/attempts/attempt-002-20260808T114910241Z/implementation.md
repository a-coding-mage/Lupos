# S016315 implementation evidence (attempt 2)

- Pipeline: P01; lease owner: `codex-root-p01`; status at implementation: `IN_PROGRESS`.
- Source: `vendor/linux/include/uapi/linux/nfsacl.h` at Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Destination: `src/include/uapi/linux/nfsacl.rs`.
- Architectures: `common` (selected by both frozen x86_64 and AArch64 configurations).
- The complete pinned header was read. Its include guard `_UAPI__LINUX_NFSACL_H` is represented by the Rust module boundary; no conditional branches remain after the guard.
- All 16 selected macro/guard records were mechanically accounted for: guard `_UAPI__LINUX_NFSACL_H`; `NFS_ACL_PROGRAM`; `ACLPROC2_NULL`, `ACLPROC2_GETACL`, `ACLPROC2_SETACL`, `ACLPROC2_GETATTR`, `ACLPROC2_ACCESS`; `ACLPROC3_NULL`, `ACLPROC3_GETACL`, `ACLPROC3_SETACL`; `NFS_ACL`, `NFS_ACLCNT`, `NFS_DFACL`, `NFS_DFACLCNT`, `NFS_ACL_MASK`; and `NFS_ACL_DEFAULT`.
- All 15 numeric macros preserve their Linux names and values as public `u32` constants. No functions, types, statics, operative configuration branches, ABI layouts, locks, lifetimes, or unsafe operations are present in this UAPI header.
- Direct pinned contexts inspected: `include/linux/nfsacl.h`, NFSv3 client/XDR/ACL users, NFS server ACL users, and `fs/nfsd/nfssvc.c`; they consume the preserved names and values.
- No compiler, formatter, linker, test, runtime, Git mutation, or historical Lupos source was used.

Frozen evidence hashes:

```text
phase0  0123af9e96b58af4e98b91a2de5608d36b63c67864b9041d3340587f3dbe40d2
scope   b83349e6731e17e5da5e04a0ea053487e8ac8d9269538dbdb367d83f77b17e0a
symbols 7484d7b0dd80f45e18d726b04752827fe28555cc5c1af0e127948380e6688abf
abi     ae0caca34fe9b6eb8097958d2fbb8d2b6a1fad60f91f3c2b8d948c43dbfcee39
lifetimes 0e7e60940dd21c28f3d10965325f70644fc000495a2d61984437dec666da93d8
queue   cfa8dcf0d81c408de38d4202fe1ce9d9e5e301d1e67439c9530da94b1e64ee3f
```
