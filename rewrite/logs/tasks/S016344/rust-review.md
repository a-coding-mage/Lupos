# Rust source review — S016344 (slot 2)

Scope reviewed: `include/uapi/linux/psp.h` at pinned revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the current candidate
`src/include/uapi/linux/psp.rs`, its candidate record, and the task-owned
semantic-closure proposal. This was a manual source review only; no compiler,
formatter, test, runtime command, historical source, or diagnostic output was
used.

## Finding RUST-S016344-001 — C string-literal macros changed to Rust fat strings

**Severity: reject / requires source correction.**

`PSP_FAMILY_NAME`, `PSP_MCGRP_MGMT`, and `PSP_MCGRP_USE` are C string-literal
macros in the pinned UAPI header (lines 10, 95, and 96). A C use of each macro
has a terminating NUL and decays or initializes as a C character sequence. The
candidate declares all three as `&str` (Rust lines 7, 75, and 76). A Rust
`&str` is a two-word slice (pointer plus length), is not a C character array,
and does not guarantee an appended NUL. It therefore cannot faithfully serve
the C ABI use of a macro such as `PSP_FAMILY_NAME`: pinned
`net/psp/psp-nl-gen.c:161-163` initializes `struct genl_family.name`, whose
definition is `char name[GENL_NAMSIZ]` in
`include/net/genetlink.h:78-82`.

The final translation must retain an explicitly NUL-terminated C-compatible
static byte representation (and use it without creating a Rust `&str` FFI
substitute), while preserving the macro spelling and byte content. The same
representation issue applies to both multicast-group macros even though the
pinned implementation currently writes their matching literals directly.

Associated semantic-closure records:

- `SC1-c5ec26d376cd91e7419bc684625006bbcdf0f3d9045b41b1c2ce144839746ed3`
- `SC1-6357a7d0a31fe37bc52817bd35d302b1dfde952936491132e7deaade9819ee36`
- `SC1-7129d129eeb095fc3b0b86c482b06a77a38fa698c86bb52e53799a9aac3a6766`
- `SC1-2100f8b9f0c7500dcdbd1fa0f1daf9aa1aba31114cfc4d8fc299a7ea26ae478b`
- `SC1-0bbd9981ae01ebf21b49c4b16d0b8e42dd0a02053c976ec31d02eb2e16980c7a`
- `SC1-f74106fc3b608d9e3356101c513dfe0c891457590ea49d59996c7acb2798d934`

## Checked without finding

- `psp_version` has source ordinals 0 through 3; the candidate's `#[repr(C)]`
  enum preserves both those values and the C enum representation requested by
  the source.
- Every anonymous-enum identifier is represented as `core::ffi::c_int`, which
  matches the C integer-constant type and preserves all explicit starting
  values, implicit increments, and `__MAX - 1` aliases.
- `PSP_FAMILY_VERSION` is correctly represented as a `c_int` value of 1.
- The header has no pointers, fields, unsafe blocks, allocation, callbacks,
  aliasing, synchronization, `Drop`, pinning, or `Send`/`Sync` surface to
  audit. No additional ownership or borrow-duration finding arises.

Result: **FINDINGS**. The candidate must be corrected and independently
reviewed again before application; the sealed proposal is mechanically bound
to this candidate and contains no unresolved proposal decision, but it does
not establish the missing C-string representation.
