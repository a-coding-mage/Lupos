# Parity review — S016142 (slot 1)

Reviewed only `vendor/linux/include/uapi/linux/handshake.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the current
`src/include/uapi/linux/handshake.rs`, the task candidate snapshot, frozen
S016142 inventory/ABI/lifetime records, and direct pinned handshake callers
and headers. No compiler, formatter, test, diagnostic, historical source, or
other task evidence was used.

## Result: findings require application

### P1 — C-string macro contract is replaced with Rust fat slices

Linux symbols: `HANDSHAKE_FAMILY_NAME`, `HANDSHAKE_MCGRP_NONE`, and
`HANDSHAKE_MCGRP_TLSHD`.

Evidence: `include/uapi/linux/handshake.h:10,73-74` defines each as a C string
literal macro. Expansion supplies a NUL-terminated character array and, in an
expression, a pointer-compatible character sequence. The candidate instead
declares all three as `&str` (`src/include/uapi/linux/handshake.rs:7,49-50`),
which is a Rust UTF-8 slice (data pointer plus length) and does not include the
C terminating NUL. This changes both value representation and C-facing use.
The direct pinned caller `net/handshake/genl.c:50-52` initializes
`struct genl_family.name` with `HANDSHAKE_FAMILY_NAME`; its declared destination
is `char name[GENL_NAMSIZ]` in `include/net/genetlink.h:78-81`. The candidate
cannot preserve that initializer/array-byte contract as written. Supply
NUL-terminated byte representations and an exact C-compatible use boundary
for every affected macro; do not use `&str` as the UAPI representation.

### P1 — named C enumerators no longer have their declared global integer-constant interface

Linux symbols: `HANDSHAKE_HANDLER_CLASS_NONE`, `HANDSHAKE_HANDLER_CLASS_TLSHD`,
`HANDSHAKE_HANDLER_CLASS_MAX`, `HANDSHAKE_MSG_TYPE_UNSPEC`,
`HANDSHAKE_MSG_TYPE_CLIENTHELLO`, `HANDSHAKE_MSG_TYPE_SERVERHELLO`,
`HANDSHAKE_AUTH_UNSPEC`, `HANDSHAKE_AUTH_UNAUTH`, `HANDSHAKE_AUTH_PSK`, and
`HANDSHAKE_AUTH_X509`.

Evidence: `include/uapi/linux/handshake.h:13-30` declares these as C
enumerators. They are unqualified file-scope integer constant expressions in
the UAPI namespace. The candidate retains the spellings only as associated
variants of `handshake_handler_class`, `handshake_msg_type`, and
`handshake_auth` (`src/include/uapi/linux/handshake.rs:10-28`), so it exports
no same-level constants and replaces direct integer expressions with distinct
Rust enum values requiring qualification/conversion. The direct caller
`net/handshake/request.c:112-119` compares the `int` field
`handshake_proto.hp_handler_class` (declared at
`net/handshake/handshake.h:54-57`) directly against
`HANDSHAKE_HANDLER_CLASS_NONE` and `HANDSHAKE_HANDLER_CLASS_MAX`; this is the
original integer-constant contract, not an enum-object contract. The candidate
therefore omits the UAPI names/usage semantics required by direct callers.
Provide top-level, exact-width integer constants for these enumerators (while
retaining any necessary enum type representation) so uses retain their Linux
names and integer-expression behavior.

## Checked without additional finding

The candidate preserves the numeric values and computed maxima for all four
anonymous enum groups (`HANDSHAKE_A_X509_*`, `HANDSHAKE_A_ACCEPT_*`,
`HANDSHAKE_A_DONE_*`, and `HANDSHAKE_CMD_*`) as `i32`, and preserves
`HANDSHAKE_FAMILY_VERSION == 1`. The source has no functions, storage,
allocation, locking, RCU, refcount, cleanup, or error paths. No branding delta
was observed. The frozen S016142 records cover both x86_64 and aarch64 and
remain `PENDING_REVIEW`; the two findings above are source-level UAPI/ABI
interface defects requiring resolution before those records can be closed.
