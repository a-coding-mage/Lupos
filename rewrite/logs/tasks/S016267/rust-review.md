# Rust review — S016267 attempt 1, slot 2

Reviewer role: Rust semantics reviewer (slot 2).  This was a manual,
source-only review; no compiler, formatter, linker, test, debugger, or
rust-analyzer diagnostic was used.

Reviewed fixed inputs:

- pinned source: `vendor/linux/include/uapi/linux/netdev.h` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df`;
- candidate digest (the required candidate artifact):
  `rewrite/logs/tasks/S016267/candidate.diff` =
  `f24cb1108af94200ee7a600fac46c70a86ad7f5182fa08b8da014df602b81fd2`;
- implementation digest:
  `a67200b9de936585f98f6aed0c4e4f7733d3ba0c36f4496d97e583cb7caf9ac6`;
- semantic-closure proposal digest:
  `92fb6526a3acea75ba3400fb8defb26a39201c1cbb9389d80ef1c2dcdf3a4810`.

## Finding RUST-S016267-1 — reject: C string-literal macros became non-C ABI `&str` values

`NETDEV_FAMILY_NAME`, `NETDEV_MCGRP_MGMT`, and
`NETDEV_MCGRP_PAGE_POOL` are C preprocessor macros whose replacement tokens
are string literals (`netdev.h:10,249-250`).  A C string literal has a
trailing `NUL`, array-literal semantics, and can initialize a C `char` array.
The kernel demonstrably uses `NETDEV_FAMILY_NAME` in the `.name` initializer
of `struct genl_family` (`net/core/netdev-genl-gen.c:264`); that field is
`char name[GENL_NAMSIZ]` (`include/net/genetlink.h:78-81`).

The candidate instead declares each macro as `&str`
(`src/include/uapi/linux/netdev.rs:7,172-173`).  A Rust `&str` is a UTF-8,
non-NUL-terminated fat reference (data pointer plus length), is neither the
literal byte array nor a C `char[N]` initializer, and therefore cannot
preserve the macro's UAPI/FFI representation.  This must be represented with
an explicitly NUL-terminated C-compatible byte-array form and a use-site
mapping that preserves C array-initializer behavior; do not close the
associated macro records as `COMPLETE` until that representation is reviewed.

Affected current semantic-proposal selection-expression keys:

- `SC1-2b0780cf961cecf9f362d8a378aede1a07382de8bb261f6f815e61a06dc46d36`
  (`NETDEV_FAMILY_NAME`, aarch64)
- `SC1-7582c7ee037cf90768992d806553d15729abea023d4e22178632fce533045dfd`
  (`NETDEV_FAMILY_NAME`, x86_64)
- `SC1-f2ec8c26ac9a162087526007207ae1c58d9ef5531bfa6235b4d4a6190cf4bb48`
  (`NETDEV_MCGRP_MGMT`, aarch64)
- `SC1-1c250d20e165b164dd7cf9ef6dae8c63f66c69e008f3b5aa29a4243cd7e8913a`
  (`NETDEV_MCGRP_MGMT`, x86_64)
- `SC1-70ffab2cdccc6c3f2ac8d0c06a91002ca0105404b0941de49f1be7a41bb6059c`
  (`NETDEV_MCGRP_PAGE_POOL`, aarch64)
- `SC1-496721e28a8ebc548130ec415e8a41d3ac9badcda5a0ab48417d56b54246c6ae`
  (`NETDEV_MCGRP_PAGE_POOL`, x86_64)

No further Rust-specific finding: this header defines no structs, bitfields,
unions, functions, globals, `unsafe` blocks, allocation, or panic path.  The
integer enum constants and their explicit/successor/max-minus-one arithmetic
are represented as signed 32-bit constants with the source values, including
the `-1` XSK maximum; no independent type/layout defect was identified from
the fixed source evidence.

## Disposition

Rejected pending resolution of RUST-S016267-1.  This report is complete for
slot 2 and is independent of the parity review.
