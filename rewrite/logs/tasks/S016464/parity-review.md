# Parity review: S016464 (slot 1)

Reviewed `src/include/uapi/linux/virtio_ids.rs` against pinned
`vendor/linux/include/uapi/linux/virtio_ids.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` for the common x86_64/AArch64
scope.

## Finding P1 — upstream BSD notice was not retained

`virtio_ids.h:5-30` contains the complete BSD redistribution, attribution,
and disclaimer notice (including IBM attribution).  The Rust candidate only
paraphrases that the definitions are BSD licensed (`virtio_ids.rs:9-10`), and
does not retain that relevant upstream notice.  This violates the rewrite
source-tree requirement to retain relevant upstream copyright notices.  The
applier should preserve the complete upstream BSD notice as a Rust comment
after the immutable provenance block; the required GPL provenance line itself
must remain unchanged.

## Verified parity

- All 47 object-like identifier macros are present as public `c_int` constants:
  40 `VIRTIO_ID_*` values and 7 `VIRTIO_TRANS_ID_*` values.
- Identifier spellings, grouping, order, and literals match exactly, including
  normal-ID gaps `13 -> 16` and `41 -> 45`, plus transitional values
  `0x1000..0x1005` and `0x1009`.
- Every C literal is an unsuffixed `int` literal and all values are representable
  by `c_int` on both approved architectures; no functions, layouts, state,
  allocation, locking, cleanup, or conditional selected content exists beyond
  the C-only include guard.
- The immutable provenance records the exact source path, pinned Linux SHA,
  common architecture scope, and task ID. No branding delta, placeholder, or
  Rust test was found.

No build, formatter, test, or runtime command was run.
