# Resolution — S016267 attempt 1

Applier review was manual and source-only. No compiler, formatter, linker,
test, debugger, runtime command, or diagnostic was used.

## P01-S016267-PARITY-1 and RUST-S016267-1 — accepted; task blocked

Pinned `include/uapi/linux/netdev.h:10,249-250` defines
`NETDEV_FAMILY_NAME`, `NETDEV_MCGRP_MGMT`, and `NETDEV_MCGRP_PAGE_POOL` as C
string-literal macros. Each literal includes a trailing NUL and is an array
expression. The current Rust `&str` constants do neither: they omit the NUL
and are fat string-slice references. The family-name use at
`net/core/netdev-genl-gen.c:264` initializes `struct genl_family.name`, which
is `char name[GENL_NAMSIZ]` at `include/net/genetlink.h:78-81`; C applies the
literal's array-initializer and zero-fill semantics at that use.

No frozen source-level rule or task-local ABI/lifetime record establishes the
Rust representation and every-use mapping that preserves both the exact bytes
and that C array-initializer behavior. Replacing the constants with a Rust
byte array or `static` would still require a new, reviewed mapping for their
aggregate-initializer use and would change the sealed candidate. It is not
faithful to choose that design here without the missing contract. The six
string-macro selection-expression records named by both reviewers therefore
cannot truthfully remain closed for this candidate.

Disposition: accept both findings; BLOCKED pending an authoritative Rust
C-string-literal/macro and aggregate-initializer mapping, including the
required representation at every selected use.

## P01-S016267-PARITY-2 — accepted as an evidence-closure defect

Pinned `netdev.h:7-8,252` uses `_UAPI_LINUX_NETDEV_H` only as a C
preprocessor single-inclusion guard. Rust module declarations, rather than an
item in this file, are the corresponding mechanism; no Rust value or symbol
for this macro is justified. The sealed proposal nevertheless marks its two
selection-expression records `COMPLETE` without recording that mapping.

Disposition: the source needs no invented guard item, but the closure record
must be regenerated with the explicit Rust-module single-inclusion mapping.
That regeneration is also required after the string-literal mapping changes
the candidate.

The candidate was not edited. Its sealed proposal and both reviews remain
evidence of the rejected attempt and must not be reused to close a changed
candidate.
