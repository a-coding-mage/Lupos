# Rust source review — S016142 (slot 2)

Reviewed only the current candidate `src/include/uapi/linux/handshake.rs` against
the pinned `vendor/linux/include/uapi/linux/handshake.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, plus its direct selected consumers
(`net/handshake/genl.c`, `net/handshake/netlink.c`, `net/handshake/request.c`,
and `net/handshake/tlshd.c`) and frozen task records. No compiler, formatter,
test, rust-analyzer diagnostic, historical Rust source, or generated archive
was used.

## Result: reject — blocking findings

### RUST-001 — C string-literal UAPI macros were changed to non-FFI Rust strings

- **Candidate:** `HANDSHAKE_FAMILY_NAME`, `HANDSHAKE_MCGRP_NONE`, and
  `HANDSHAKE_MCGRP_TLSHD` at `src/include/uapi/linux/handshake.rs:7`, `:64`,
  and `:65` are `&str` values.
- **Pinned evidence:** `handshake.h:10`, `:73`, and `:74` define C string
  literals. Each expression is an array containing a trailing NUL and, in a
  pointer context, decays to a single C-character pointer; it is neither UTF-8
  constrained nor a two-word Rust fat pointer. `net/handshake/genl.c:55-59`
  initializes the Generic Netlink family name from `HANDSHAKE_FAMILY_NAME`;
  the UAPI values also define the externally visible family and multicast-group
  spellings.
- **Why this is a Rust-semantic/ABI defect:** a Rust `&str` has pointer-plus-
  length representation and its byte sequence excludes the C terminator. It
  cannot stand in for a C string literal in FFI or in fixed C character-array
  initialization. Any later conversion either needs an additional allocation/
  temporary or silently changes the terminating-byte and pointer semantics.
- **Required resolution:** represent each macro with an exact NUL-terminated
  byte representation appropriate to every mapped C use, and do not expose
  `&str` as its ABI substitute. The applier must document the precise Rust
  representation and each direct C-use mapping.

### RUST-002 — named C enum constants lost their `int` expression semantics

- **Candidate:** the three named groups at
  `src/include/uapi/linux/handshake.rs:11-33` are Rust `#[repr(C)]` enums and
  provide the C enumerator spellings only as enum variants.
- **Pinned evidence:** `handshake.h:13-29` declares the C enum tags. Under the
  frozen C source language, its enumerators are integer constant expressions;
  direct consumers assign and compare them with `int` fields:
  `net/handshake/handshake.h:55` declares `hp_handler_class` as `int`,
  `net/handshake/tlshd.c:32-34` declares `th_type` and `th_auth_mode` as `int`,
  and `request.c:116-118`, `netlink.c:48-68`, and `tlshd.c:219-245` consume the
  enumerators as ordinary integer values.
- **Why this is a Rust-semantic/ABI defect:** a Rust variant has its enclosing
  enum type, is not an unqualified `i32` constant, and requires casts at every
  equivalent integer use. It additionally imposes Rust enum-validity rules:
  arbitrary values representable by the Linux `int` fields cannot soundly be
  materialized as one of these Rust enums. `#[repr(C)]` does not make a Rust
  enum a replacement for C's integer-constant-expression interface.
- **Required resolution:** preserve the exported enumerator spellings as
  primitive integer constants for the direct integer uses. If named enum types
  are retained for another mapped interface, their representation and any FFI
  boundary must not assume only listed discriminants can occur.

### RUST-003 — enum ABI assertion remains unproven by the frozen semantic record

- **Candidate:** `#[repr(C)]` is asserted on all three named Rust enums at
  `src/include/uapi/linux/handshake.rs:10`, `:19`, and `:27`.
- **Frozen evidence:** the S016142 entries in `rewrite/ABI.tsv` and
  `rewrite/LIFETIMES.tsv` retain `PENDING_REVIEW` for all three named enums on
  both x86_64 and aarch64. The header declares no fixed-width enum underlying
  type and provides no UAPI structure or function signature that would settle
  a cross-language layout from source alone.
- **Why this is a source-review finding:** no compilation or compiler-backed
  layout diagnostic is permitted in Phase 1. The candidate therefore cannot
  treat `repr(C)` as established evidence that its layout and valid-value set
  reproduce the frozen C ABI. This is particularly material because RUST-002
  already shows the direct Linux consumers require `int` values, not enum
  objects.
- **Required resolution:** close the corresponding ABI/lifetime semantic
  records with pinned-source and frozen-manifest evidence, or remove the
  unsupported enum-FFI assertion in favor of the verified integer interface.

## Manual audit coverage

- No structs, unions, bitfields, packed/aligned objects, function pointers,
  callbacks, raw pointers, pointer arithmetic, atomics, interior mutability,
  pinning, `Send`/`Sync` assertions, allocations, bounds operations, `Drop`,
  or `unsafe` blocks occur in the candidate. Those categories have no positive
  candidate implementation to approve.
- The anonymous C enums are represented as `i32` constants with the pinned
  values and subtraction expressions; this portion does not introduce a
  separate ownership or arithmetic finding.
- `HANDSHAKE_FAMILY_VERSION: i32 = 1` preserves the C integer literal value.
  The string constants and named enum groups prevent acceptance of the file as
  a whole.
