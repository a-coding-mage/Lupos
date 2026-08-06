# Rust review — S013677 (slot 2)

Result: **REJECT — correct the `decompress_method` safety contract before
acceptance.**

Reviewed the complete pinned
`vendor/linux/include/linux/decompress/generic.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the implementation at
`vendor/linux/lib/decompress.c:63-87`, fresh candidate
`src/include/linux/decompress/generic.rs`, the task records for both frozen
targets, and the selected pinned consumers in `init/initramfs.c:519-574` and
`init/do_mounts_rd.c:301-314`. No source or manifest was edited, and no build,
format, test, or runtime command was run.

## Finding

1. **Medium — the declaration documents a stricter and incomplete
   `decompress_method` precondition than the pinned implementation.**

   The candidate says that `inbuf` “must designate `len` readable bytes” and
   that a non-null `name` receives null only when fewer than two bytes are
   available (`generic.rs:37-39`). Pinned `decompress_method` first tests
   `len < 2`; on that path it does not dereference `inbuf`, stores null through
   a non-null `name`, and returns null (`lib/decompress.c:68-71`). Thus it
   deliberately accepts a null `inbuf` for every `len < 2`, including negative
   values. For `len >= 2`, the implementation reads only `inbuf[0]` and
   `inbuf[1]` (`:74` and `:78`), rather than requiring `len` readable bytes.
   It also stores null through `name` when two or more bytes have an unknown
   magic, because the sentinel has `.name = NULL` (`:51-60`, `:83-87`).

   Correct the Rust declaration’s safety documentation to express those exact
   conditions, without imposing a stronger Rust-side contract. Raw pointers
   remain the correct representation; this is a declared FFI precondition,
   not a reason to create references or slices.

## Checks that passed

- `decompress_fn` is correctly a nullable `Option<unsafe extern "C" fn(...)>`:
  both a C function-pointer typedef and the null return from
  `decompress_method` are representable without manufacturing an invalid
  non-null Rust function pointer. `Option` is also used for all three callback
  pointer arguments, preserving the C null-pointer ABI.
- The callback ABI exactly preserves parameter order and C widths on both
  frozen LP64 targets: mutable `unsigned char *`, `long`, two
  `void *`/`unsigned long` callbacks, mutable output and position pointers,
  mutable `char *` error callback, and `int` result. `unsafe extern "C"`
  appropriately leaves validation of every raw-pointer/callback invocation at
  the kernel boundary; the candidate creates no references, ownership,
  `Send`/`Sync`, `Drop`, or allocation claim.
- `decompress_method` preserves `const unsigned char *` as `*const c_uchar`,
  preserves the nullable writable `const char **` output as `*mut *const
  c_char`, uses the correct C ABI and `c_long` width, and returns the nullable
  `decompress_fn` representation. The candidate contains no unsafe block, so
  no local `SAFETY` comment is required.
- The header has no selected architecture/Kconfig branch beyond its include
  guard; the candidate adds no `cfg` divergence and has correct immutable
  provenance for `common`.

The applier must also close the existing S013677 ABI/lifetime records for both
frozen targets before `DONE`. No source edits were made by this reviewer.
