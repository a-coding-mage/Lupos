# Parity review — S014160 (slot 1)

Reviewed only the pinned source `vendor/linux/include/linux/kasan-tags.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the candidate
`src/include/linux/kasan-tags.rs`, and the frozen x86_64/aarch64 configuration
records.  The branch is `feat/bun-like-rewrite-test`; the queue row is
`REVIEWING`, maps the Linux header to the candidate path, and is scoped as
`common`.

## Finding P1 — required immutable SPDX provenance does not use `GPL-2.0-only`

The candidate starts with `// SPDX-License-Identifier: GPL-2.0`, while the
project's required immutable Rust provenance header prescribes
`// SPDX-License-Identifier: GPL-2.0-only`.  Update only that provenance line
to the mandated form.  The remaining provenance fields are exact: source path,
Linux revision, `common` architecture membership, and task ID `S014160`.

## Source parity checked

- `KASAN_TAG_KERNEL`, `KASAN_TAG_INVALID`, and `KASAN_TAG_MAX` preserve the
  C unsuffixed `int` literal values `0xFF`, `0xFE`, and `0xFD` as `i32`.
- The frozen x86_64 and aarch64 configs each state `# CONFIG_KASAN is not set`;
  neither defines `CONFIG_KASAN_HW_TAGS`.  Therefore the active C branch is
  `KASAN_TAG_MIN = 0x00`, which the candidate implements.  The compiled-out
  hardware-tags branch is `0xF0` and is correctly not selected by either
  frozen configuration.
- `_LINUX_KASAN_TAGS_H` is solely a C preprocessor include guard, with no
  runtime/API value to reproduce as a Rust item.  The candidate introduces no
  extra state, branching, ABI, or behavioral substitution.

No compiler, formatter, rust-analyzer, build, test, or runtime diagnostic was
run.  No source or queue file was modified.
