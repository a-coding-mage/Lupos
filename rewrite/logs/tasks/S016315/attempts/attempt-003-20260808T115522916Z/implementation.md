# Implementation evidence — S016315

- Task: `S016315`
- Pipeline/attempt: `P01` / `3`
- Linux source: `vendor/linux/include/uapi/linux/nfsacl.h`
- Destination: `src/include/uapi/linux/nfsacl.rs`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architectures: `common` (frozen x86_64/AArch64 union)
- Phase-0 identity: `0123af9e96b58af4e98b91a2de5608d36b63c67864b9041d3340587f3dbe40d2`
- Queue fingerprint: `cfa8dcf0d81c408de38d4202fe1ce9d9e5e301d1e67439c9530da94b1e64ee3f`
- Scope: `b83349e6731e17e5da5e04a0ea053487e8ac8d9269538dbdb367d83f77b17e0a`
- Symbols: `7484d7b0dd80f45e18d726b04752827fe28555cc5c1af0e127948380e6688abf`
- ABI: `ae0caca34fe9b6eb8097958d2fbb8d2b6a1fad60f91f3c2b8d948c43dbfcee39`
- Lifetimes: `0e7e60940dd21c28f3d10965325f70644fc000495a2d61984437dec666da93d8`

## Pinned source read

The complete pinned `include/uapi/linux/nfsacl.h` was reread.  It contains
the GPL syscall-note SPDX expression, the `File: linux/nfsacl.h` notice, the
`(C) 2003 Andreas Gruenbacher <agruen@suse.de>` notice, the
`_UAPI__LINUX_NFSACL_H` guard, and fifteen integer object-like macros.  The
sixteen selected guard/macro semantics are represented: the guard boundary,
`NFS_ACL_PROGRAM`, five ACLPROC2 values, three ACLPROC3 values, and six ACL
flag values.  No types, functions, storage, or architecture-specific branch
exists in the pinned header.

## Translation decisions

The Rust destination is a fresh path-preserving module.  Every selected
object-like macro is a public `i32` constant with the exact pinned numeric
value.  The C include guard is expressed by the Rust module boundary and does
not create a runtime symbol.  The source copyright and SPDX notices and
immutable provenance are retained.  No unsafe code, allocation, locking,
cleanup, or lifetime conversion is required by this header.

No compiler, formatter, linker, test, emulator, debugger, runtime, or Git
history command was used.
