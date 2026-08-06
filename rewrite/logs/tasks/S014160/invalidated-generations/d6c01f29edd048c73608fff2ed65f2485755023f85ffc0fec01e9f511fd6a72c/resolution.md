# Resolution — S014160

Applier source-only review against pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` is complete.  No compiler,
formatter, analyzer, build, link, test, runtime, debugger, or historical Rust
source was used.

## Review finding disposition

### P1 — provenance SPDX identifier

**Disposition: fixed.**  The parity review correctly identified that the
candidate used `GPL-2.0` while the immutable Rust provenance rule requires
`GPL-2.0-only`.  `src/include/linux/kasan-tags.rs:1` now uses exactly
`// SPDX-License-Identifier: GPL-2.0-only`.  The remaining immutable
provenance fields continue to identify `include/linux/kasan-tags.h`, revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`, architecture membership `common`,
and task `S014160`.

## Independent upstream and frozen-configuration confirmation

The complete upstream header has only a preprocessor include guard and four
integer macro definitions:

- `KASAN_TAG_KERNEL = 0xFF`, `KASAN_TAG_INVALID = 0xFE`, and
  `KASAN_TAG_MAX = 0xFD` are unconditional unsuffixed C `int` literals.  The
  Rust `i32` constants preserve their values and signed integer type on both
  frozen targets.
- Upstream line 9 selects `KASAN_TAG_MIN = 0xF0` only when
  `CONFIG_KASAN_HW_TAGS` is defined; its line 11 `#else` selects `0x00`.
  Both `rewrite/configs/x86_64/frozen.config` and
  `rewrite/configs/aarch64/frozen.config` explicitly leave `CONFIG_KASAN`
  unset and contain no `CONFIG_KASAN_HW_TAGS` definition.  Thus the selected
  configuration union compiles out the hardware-tag branch and selects
  `KASAN_TAG_MIN: i32 = 0x00`, exactly as represented by the candidate.
- `_LINUX_KASAN_TAGS_H` is an include guard only.  It has no Rust runtime,
  ABI, ownership, storage, linkage, cleanup, lock, refcount, RCU, or unsafe
  semantic to reproduce.

## Pending-record closure

For both x86_64 and aarch64, the `SYMBOLS.tsv` pending entries are resolved by
this task as follows: the include-guard condition is represented by Rust module
loading; `KASAN_TAG_KERNEL`, `KASAN_TAG_INVALID`, and `KASAN_TAG_MAX` map to
the corresponding `i32` constants; the `CONFIG_KASAN_HW_TAGS` `#ifdef` and
its `0xF0` definition are compiled out; and the `#else` `KASAN_TAG_MIN = 0x00`
maps to the candidate constant.  This header has no ABI, lifetime, ownership,
or driver-ABI decision to retain as pending.

Both review reports are accepted after P1's correction.  No unresolved finding
or semantic record remains for S014160.
