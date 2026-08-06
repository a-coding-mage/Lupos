# Resolution — S016395

Reviewed the complete pinned `include/uapi/linux/sunrpc_netlink.h` and the
selected SunRPC consumers cited by both reviewers. No build, formatter, test,
or runtime command was run.

## P-1 / RUST-2 — named-enum enumerator expressions

**Resolved in source.** C11 6.7.2.2 gives enumeration constants type `int`;
the named enum tag is a separate object-type namespace. The candidate's
transparent wrapper made `SUNRPC_CACHE_TYPE_IP_MAP` and
`SUNRPC_CACHE_TYPE_UNIX_GID` non-integral Rust values, which could not preserve
the selected `u32` call/bitwise contexts in `cache.c` and `svcauth_unix.c`.
They are now `c_int` constants. The separately addressable tag mapping is
`pub type sunrpc_cache_type = c_int`.

## P-2 / RUST-1 — string-literal macro surfaces

**Resolved in source.** `SUNRPC_FAMILY_NAME`, `SUNRPC_MCGRP_NONE`, and
`SUNRPC_MCGRP_EXPORTD` are now public immutable, NUL-terminated `[u8; N]`
statics. This retains the literal array lengths and bytes for indexing,
array-initializer translation, and length contexts; the public `*_PTR` views
retain ordinary-expression pointer decay. `u8` follows the selected SunRPC
Kbuild commands' `-funsigned-char` setting. In particular, the later
translation of `net/sunrpc/netlink.c` can construct the zero-filled
`genl_family.name` array from the public seven-byte family-name array rather
than being limited to a raw pointer.

## RUST-3 — frozen enum object ABI and lifetime records

**Resolved in task records.** The 16 `S016395` rows in `rewrite/ABI.tsv` (the
named enum plus seven anonymous enums on each target) now record a four-byte,
four-byte-aligned signed C `int` enum representation, `NOT_APPLICABLE` export
kind, and `COMPLETE` status. Evidence for each target points to its pinned
SunRPC `cache.o` command: LLVM 19, the exact target triple, GNU11, and no
`-fshort-enums`; every enumerator in this header is in the signed C `int`
range.

The matching 16 `S016395` rows in `rewrite/LIFETIMES.tsv` now record that
these declarations introduce no object or storage, ownership/lifetime/locking
are `NOT_APPLICABLE`, and their status is `COMPLETE`. No `S016395` symbol rows
were changed: they are not part of the enum ABI/lifetime blocker addressed by
this requeue.

The task is requeued rather than marked `DONE` so that the corrected source
and newly completed records receive a fresh independent review cycle.
