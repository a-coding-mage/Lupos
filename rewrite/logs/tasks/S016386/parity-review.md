# S016386 parity review (slot 1)

Reviewed the complete pinned `include/uapi/linux/socket.h` at revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/uapi/linux/socket.rs`, the S016386 scope/symbol records, and the
frozen x86_64/AArch64 Kbuild command metadata.

## Finding P1 — reject: `__data` has the wrong signed element type

**Severity:** blocking

Upstream declares `__data` as `char[_K_SS_MAXSIZE - sizeof(unsigned short)]`
at `vendor/linux/include/uapi/linux/socket.h:21`.  The frozen Kbuild command
metadata for both selected architectures contains `-funsigned-char` (for
example `rewrite/metadata/x86_64/compile_commands.json:3` and
`rewrite/metadata/aarch64/compile_commands.json:3`); the pinned global rule is
also explicit at `vendor/linux/Makefile:607`.  Therefore the selected C field
is an array of 126 **unsigned** character objects.  The candidate instead
declares `[i8; ...]` at `src/include/uapi/linux/socket.rs:30`.

The byte size and offsets coincide, but the signedness is part of the exposed
field's semantics: reads, promotion, comparisons, and assignments for values
above `0x7f` differ.  This is a UAPI structure used as a socket-address buffer
in selected TCP, MPTCP, RDMA, and multicast UAPI records.

**Required resolution:** represent the member as `[u8; 126]` (or an exact
equivalent that preserves its unsigned-character semantics) while retaining
the required 128-byte size, offset zero for `ss_family`, and pointer-derived
alignment.

## Finding P2 — reject: named wrapper aggregates remove C anonymous-member promotion

**Severity:** blocking

The source declares an anonymous union containing an anonymous struct at
`vendor/linux/include/uapi/linux/socket.h:16-25`.  GNU C consequently exposes
`ss_family`, `__data`, and `__align` as direct members of
`struct __kernel_sockaddr_storage`; the union and inner struct names do not
exist in the public source interface.  The candidate instead introduces two
public named types and makes the outer struct contain the one named field
`__anonymous_union` at `src/include/uapi/linux/socket.rs:26-49`.  Consumers
must now traverse `.__anonymous_union.__anonymous_struct.ss_family` (and use
Rust union access) rather than accessing the promoted source members.

Matching only the 128-byte, 8-byte-aligned memory representation does not
preserve this UAPI aggregate's fields, source-level member paths, or its
member-access behavior.  The candidate's statement that the fields are
"otherwise preserved exactly" is thus false.  The corresponding semantic and
ABI records for the anonymous union and struct remain `PENDING_REVIEW` in
`rewrite/{SYMBOLS,ABI,LIFETIMES}.tsv` and must be closed before `DONE`.

**Required resolution:** establish a call-site-preserving Rust representation
of the anonymous union/struct and their promoted members, or escalate/block if
Rust cannot express that contract without changing its selected consumers.  A
private layout helper may be necessary, but adding a new public nesting level
is not equivalent.

## Finding P3 — reject: `_K_SS_MAXSIZE` changes the object-like macro expression type

**Severity:** blocking

`_K_SS_MAXSIZE` is the unsuffixed decimal integer literal macro `128` in
`vendor/linux/include/uapi/linux/socket.h:8`; on both frozen targets that
replacement list has C `int` type.  The candidate publishes `usize` at
`src/include/uapi/linux/socket.rs:10-14`, explicitly changing the expression
type to accommodate an array bound.  This changes integer conversions,
arithmetic, signed comparisons, and the public translation of a selected
operative macro.  It cannot be justified by one internal array-length use.

The six remaining socket lock/rehash macros are correctly represented as
32-bit signed integer values, and `SOCK_BUF_LOCK_MASK` retains value `3`.

**Required resolution:** retain `_K_SS_MAXSIZE` as the semantic counterpart of
the C `int` expression and use a separate, derived Rust-only array-length
constant if Rust requires `usize` for the array type.  Document the final
mapping in the task ABI/symbol resolution.

## Coverage

The review covered every line of the unconditional 38-line source header,
both frozen architectures, the typedef, the anonymous union and struct, all
seven operative value macros, provenance, SPDX identifier, and selected UAPI
consumers.  Aside from the three findings above, the candidate has the correct
source path, exact revision, `common` architecture membership, task ID, SPDX
identifier, `u16` typedef width, macro values/types for the six socket
lock/rehash constants, and pointer member required to produce the intended
8-byte alignment on both selected 64-bit ABIs.  No source files were edited;
no build, test, formatter, compiler, or runtime command was run.
