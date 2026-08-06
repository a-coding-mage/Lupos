# Implementation — S016168

Translated the complete pinned UAPI oracle
`include/uapi/linux/if_infiniband.h` to
`src/include/uapi/linux/if_infiniband.rs` at Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The header has one unconditional operative declaration:
`INFINIBAND_ALEN`, an unsuffixed C integer constant with value 20.  It is
represented as the public `i32` constant `INFINIBAND_ALEN`, preserving the
C `int` expression type and exact value.  The C include guard has no Rust
runtime or ABI counterpart.  There are no functions, layouts, storage,
configuration branches, ownership, locking, allocation, or error paths.

The frozen common x86_64/AArch64 header-closure evidence names
`net/ipv6/addrconf.c` as the selected consumer; its
`addrconf_ifid_infiniband` path compares `dev->addr_len` with this value before
copying the interface identifier.  The immutable provenance records the task
category as `common`, as required by the canonical queue row.

Lease, queue fingerprint, pinned revision, and resolved Phase 0 identity were
verified before editing. No historical Rust source, compiler, formatter, test,
runtime command, or non-leased source file was used or changed.
