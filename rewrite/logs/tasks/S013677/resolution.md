# Application resolution — S013677 / attempt 1

Applier: `gpt-5.6-terra` / high. This resolution reopened the complete pinned
`include/linux/decompress/generic.h`, its direct pinned consumers
`lib/decompress.c`, `init/initramfs.c`, and `init/do_mounts_rd.c`, the
candidate and frozen diff, both independent reports and attestations, and the
frozen symbol/ABI/lifetime records. No compiler, formatter, analyzer, linker,
test, or runtime command was used.

## Review dispositions

- Parity review — **APPROVE, no findings**: confirmed. The C typedef's mutable
  byte buffers, LP64 `long`/`unsigned long` scalars, nullable callbacks and
  `posp`, and `int` return are represented without Rust references or
  ownership. `lib/decompress.c` conditionally writes the nullable name slot
  and returns a callback or null; the two init consumers handle those states.
  `Option<unsafe extern "C" fn>` preserves each nullable function-pointer
  position, and `*mut *const c_char` preserves writable `const char **`.
- Rust review — **APPROVE, no findings**: confirmed. The declaration retains
  the const input pointer, C ABI, and C-controlled aliasing, nullability, and
  lifetime. This header has no aggregate layout or owned resource. Its C
  include guard has no Rust runtime/ABI counterpart; the definition's `__init`
  annotation is not part of this header declaration.

## Semantic-record closure

All 29 proposal records bind candidate diff SHA-256
`365fd3d490fd102efa6071c062160b37c0ed292ba0937e62682d803d39ce2a25`,
implementation evidence SHA-256
`07c21f7adb2b764b6fd12bb78f02a5bb892e66860505380637bbf7bbd407533a`,
and proposal SHA-256
`ffcbdb094deeb153ca2010f05db2100738df59c890e54a59e412c5c4d7b02a77`.
Both independent attestations approve that same proposal. The exact source
evidence establishes selection, callback ABI, nullability, externally supplied
callback lifetime, and the absence of a locking/refcount/ownership protocol on
both architectures. Each field may therefore be committed as `COMPLETE`; no
task field remains `PENDING_REVIEW`.

No candidate edit was needed or made. This is source-only acceptance and makes
no build, link, boot, test, or compatibility claim.
