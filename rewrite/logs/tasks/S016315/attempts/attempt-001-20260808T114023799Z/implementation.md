# Implementation evidence — S016315

- Task: `S016315`
- Pipeline/attempt: `P01` / `1`
- Linux source: `vendor/linux/include/uapi/linux/nfsacl.h`
- Destination: `src/include/uapi/linux/nfsacl.rs`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architectures: `common` (the frozen x86_64/AArch64 union)
- Identity: `0123af9e96b58af4e98b91a2de5608d36b63c67864b9041d3340587f3dbe40d2`
- Queue fingerprint: `cfa8dcf0d81c408de38d4202fe1ce9d9e5e301d1e67439c9530da94b1e64ee3f`

## Source coverage

The complete pinned header was read. It contains only the SPDX/copyright
provenance, the `_UAPI__LINUX_NFSACL_H` include guard, and sixteen integer
macros: the NFS ACL program number, the NFS ACL v2/v3 procedure numbers, the
getacl/setacl mask bits, the combined mask, and the default-ACL bit. There are
no structs, enums, typedefs, conditional branches, functions, pointer values,
or architecture-specific layouts in this header.

The destination retains every exported macro name and numeric value as a
public `i32` constant. This is the C `int` representation used by the UAPI
macros and preserves the exact values consumed by the NFS client/server
procedure tables and ACL mask checks. The C include guard is represented by
the Rust module boundary and is not emitted as a runtime symbol.

Relevant pinned consumers inspected include the NFS v2/v3 ACL client and
server procedure tables and XDR mask checks in `fs/nfs`, `fs/nfsd`, and
`fs/nfs_common`; they use these values as integer indices, procedure numbers,
and bit masks without requiring additional declarations from this UAPI file.

No compiler, formatter, linker, test, emulator, debugger, or runtime command
was run.
