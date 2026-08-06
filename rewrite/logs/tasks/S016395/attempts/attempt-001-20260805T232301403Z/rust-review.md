# Rust review — S016395 (slot 2)

Reviewed only `src/include/uapi/linux/sunrpc_netlink.rs` against the complete
pinned `vendor/linux/include/uapi/linux/sunrpc_netlink.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, its selected SunRPC consumers,
and the frozen common x86_64/AArch64 records.  This was a source-only review;
no build, compiler, formatter, test, or runtime command was run.

## Result

REJECT — the candidate needs the following Rust/UAPI corrections or an
upstream-evidence-based resolution.

### RUST-1 — string-literal macros were reduced to pointer-only items (high)

The source object-like macros at
`include/uapi/linux/sunrpc_netlink.h:10,81-82` expand to string literals, not
to pointer constants.  The literals have static storage, include their NUL
byte, decay to `const char *` in ordinary expression contexts, and also retain
their array-initializer behavior.

That latter behavior is used by selected upstream code: `net/sunrpc/netlink.c`
lines 87-89 initializes `struct genl_family.name` from
`SUNRPC_FAMILY_NAME`, while `include/net/genetlink.h:78-82` declares `name` as
`char name[GENL_NAMSIZ]`.  The C string literal initializes and zero-fills that
fixed-size array.  The Rust candidate instead makes
`SUNRPC_FAMILY_NAME` a `*const c_char` (line 30) with private backing bytes.
That pointer cannot serve as the array initializer, and the private byte array
prevents the eventual Rust translation of the selected consumer from using the
source bytes to construct the required fixed array without duplicating them or
dereferencing a raw pointer.

The same pointer-only reduction affects `SUNRPC_MCGRP_NONE` and
`SUNRPC_MCGRP_EXPORTD` (lines 92 and 104): it loses `sizeof`, array-initializer,
and other string-literal contexts even though the ordinary-expression pointer
value and bytes themselves are correct.  Preserve publicly usable immutable
NUL-terminated array data as well as a controlled pointer-decay view, or
provide a documented macro-equivalent/consumer mapping that handles both the
array initialization and pointer contexts.  Do not treat a raw-pointer item as
the complete semantic equivalent of a C string-literal macro.

### RUST-2 — named-enum enumerators no longer have their GNU C integer
constant-expression type (high)

The frozen C commands for the selected SunRPC consumers use `-std=gnu11`.
Under that language mode, the enumeration constants
`SUNRPC_CACHE_TYPE_IP_MAP` and `SUNRPC_CACHE_TYPE_UNIX_GID` declared at
`sunrpc_netlink.h:13-16` are `int` integer constant expressions; the named
enum tag is a separate object type.  The candidate instead exports both
constants as the nominal transparent wrapper `sunrpc_cache_type` (lines
33-34).

This changes the operative typed interface.  For example,
`net/sunrpc/svcauth_unix.c:590,1291` passes those constants to
`sunrpc_cache_notify(..., u32 cache_type)`, and lines 842/846 use them in
bitwise operations with a `u32` mask.  C supplies the integral conversions at
those sites.  A Rust wrapper supports neither operation nor the function call
without manually reaching into `.0`, so it is not the header equivalent of
the C enumerator expression.

Keep a separately named ABI representation for `enum sunrpc_cache_type` if it
is required by a translated object declaration, but expose its enumerators as
the frozen C `c_int` integer constant expressions (or document and implement
an exact context-preserving translation at every selected use).  The anonymous
enum constants are correctly represented as `c_int` and their derived maxima
retain their values.

### RUST-3 — the enum representation is asserted without task-local ABI
evidence (medium)

The candidate asserts a frozen C `int` ABI for `enum sunrpc_cache_type` at
lines 11-17.  However, `rewrite/ABI.tsv` and `rewrite/LIFETIMES.tsv` still
leave the named enum and all seven anonymous enums `PENDING_REVIEW` for both
approved targets.  The absence of `-fshort-enums` in the recorded selected
compile commands is relevant, but is not itself a completed ABI record.

Before the applier closes this task, establish the pinned LLVM-19 x86_64 and
AArch64 representation/alignment/call treatment from the frozen command
context and record the result.  The candidate must not claim a resolved ABI
while its authoritative task records remain unresolved.

## Checked without additional findings

- The upstream dual SPDX expression, source path, revision, common architecture
  scope, and task ID are retained exactly.
- All seven anonymous enum namespaces contain every source member.  The
  explicit values, implicit increments, hidden sentinels, and public
  `*_MAX = sentinel - 1` results are numerically correct and use signed
  `c_int` arithmetic.
- All three string byte sequences are correct and include exactly one terminal
  NUL: `sunrpc\\0` (7 bytes), `none\\0` (5 bytes), and `exportd\\0` (8 bytes).
  The defect is their incomplete macro/context surface, not their storage
  duration, mutability, or byte content.
- This UAPI header has no selected Kconfig or architecture branch beyond the C
  multiple-inclusion guard.  It contains no aggregate layout, ownership,
  synchronization, callback, allocation, `unsafe`, panic, or Rust-test concern.

No source, queue, manifest, or evidence file outside this assigned Rust review
report was modified by this reviewer.
