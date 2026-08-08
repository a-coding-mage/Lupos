# Resolution — S016395 / attempt 1

This adjudication re-opened the complete pinned
`include/uapi/linux/sunrpc_netlink.h`, the current candidate and candidate
record, both review reports, the frozen queue/scope/symbol/ABI/lifetime
records, and the narrow pinned SunRPC/generic-netlink uses.  No compiler,
formatter, linker, test, runtime, or diagnostic tool was used.

## RUST-ENUM-CONSTANT-NAMESPACE-TYPE — accepted; BLOCKED pending ABI closure

The candidate's `#[repr(i32)] enum sunrpc_cache_type` is not an acceptable
sole representation.  Header lines 13--16 declare the two enumerators at
file scope with values 1 and 2.  The pinned uses treat those identifiers as
integer mask values: `net/sunrpc/cache.c:1979` accepts `u32 cache_type`, and
`net/sunrpc/svcauth_unix.c:842--847` applies both enumerators to a `u32 mask`.
The candidate instead makes the names enum variants and does not export bare
integer constants.

The source-established part of a rework is therefore exact: export bare
`SUNRPC_CACHE_TYPE_IP_MAP` and `SUNRPC_CACHE_TYPE_UNIX_GID` integer constants
with the header's values 1 and 2, and do not make a restrictive Rust enum the
only way to express either value.

The rest cannot be safely selected from the permitted evidence.  The frozen
ABI rows for `enum sunrpc_cache_type` remain `PENDING_REVIEW` for both
x86_64 and aarch64; they provide no compatible C enum representation, layout,
or alignment.  The pinned header names the C enum but does not itself fix that
ABI property.  Choosing `i32`, a Rust `repr(C)` enum, or another replacement
would guess the unresolved ABI and could alter the public UAPI type contract.

Disposition: **BLOCKED / requeue after the task's enum ABI record is closed
from allowed pinned-source evidence.**  No source correction is authorized in
this adjudication.

## RUST-C-STRING-MACRO-ABI — accepted in part; required rework is exact

`SUNRPC_FAMILY_NAME`, `SUNRPC_MCGRP_NONE`, and `SUNRPC_MCGRP_EXPORTD` are C
string-literal macros at header lines 10, 81, and 82.  Their required byte
sequences include the terminator: `sunrpc\0` (7 bytes), `none\0` (5 bytes),
and `exportd\0` (8 bytes).  The candidate's `&str` constants omit those NUL
bytes and expose a Rust data-pointer/length reference rather than the C
character-array expression and its pointer-decay behavior.  They must be
reworked to retain the exact NUL-terminated byte arrays and to provide a C
character pointer only at a consumer that requires pointer decay; `&str` is
not an equivalent exported contract.

The review's specific assertion that `net/sunrpc/netlink.c:88` initializes a
character-pointer `.name` is disproved by the pinned definition in
`include/net/genetlink.h:78--91`: `struct genl_family.name` is
`char name[GENL_NAMSIZ]`.  That initializer is an array initialization, not a
pointer-field initialization.  This does not cure the candidate: the source
macro remains a NUL-terminated C string-literal array, and no pinned use
permits replacing it with a non-NUL Rust string.  The two multicast macros
have no pinned in-tree use outside their defining header.

Disposition: **accepted correction requirement.**  Apply the stated
NUL-byte-array/pointer-decay contract during the required requeue; retain the
above correction to the review rationale.  This finding does not independently
resolve the enum ABI blocker.

## Outcome

The current candidate cannot advance to application or `DONE`.  The named C
enum ABI lacks the required frozen, source-backed closure, so the task must be
blocked and requeued only after that evidence exists.  This file changes no
source, queue state, or frozen record.
