# Parity review — S013677 (slot 1)

Reviewed the complete pinned
`vendor/linux/include/linux/decompress/generic.h` at revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/linux/decompress/generic.rs`, the frozen common/x86_64/AArch64
scope, symbol, ABI, and lifetime records, and the selected producer/consumer
context in `vendor/linux/lib/decompress.c` and
`vendor/linux/init/initramfs.c`.

## Result

PASS — no parity findings.

## Checked surface

- Provenance retains the upstream GPL-2.0 SPDX identifier, exact Linux path
  and revision, `common` architecture scope, and task ID.  No branding,
  test, placeholder, configuration branch, or runtime implementation was
  introduced.  The C multiple-inclusion guard has no Rust module counterpart.
- `decompress_fn` remains a nullable C function pointer returning C `int`.
  The candidate preserves all seven parameters, in source order: mutable
  `unsigned char *`, signed `long`, nullable `fill`, nullable `flush`, mutable
  `unsigned char *`, mutable `long *`, and nullable `error`.  The two byte
  callbacks retain mutable `void *`, `unsigned long`, and signed `long`; the
  error callback retains mutable `char *`.  `Option<unsafe extern "C" fn>` is
  the required nullable C-function-pointer representation at each pointer
  level, including the nullable top-level result.
- The declaration of `decompress_method` preserves external C linkage, its
  nullable `const unsigned char *` input, signed C `long` length, and mutable
  outer pointer / const inner pointer for `const char **name`.  The pinned
  producer returns `NULL` for fewer than two bytes and otherwise returns the
  selected decompressor pointer; the Rust return representation preserves that
  result domain.  `init/initramfs.c` passes the returned callback and its
  `NULL` fill/output-buffer arguments in the same parameter order, confirming
  the declaration's callback contract.
- Both frozen architectures select the sole type and the unconditional header
  surface.  `c_int`, `c_long`, `c_ulong`, `c_uchar`, and `c_char` preserve the
  corresponding C categories for the approved x86_64 and AArch64 targets;
  the raw pointers retain Linux-controlled lifetimes and mutability.

`rewrite/ABI.tsv` and `rewrite/LIFETIMES.tsv` still contain the Phase-0
`PENDING_REVIEW` records for `decompress_fn` on both architectures.  The
applier must close those task records before `DONE`; they do not identify a
candidate source mismatch.

No source, manifest, queue, build, format, test, or runtime action was
performed by this reviewer beyond this required review artifact.
