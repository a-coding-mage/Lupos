# Parity review: S016428

Reviewed pinned source `vendor/linux/include/uapi/linux/tty_flags.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/uapi/linux/tty_flags.rs`.

## Finding P1 — `__KERNEL__` visibility gates were removed (high)

Upstream lines 42–53 define `ASYNCB_INITIALIZED`, `ASYNCB_SUSPENDED`,
`ASYNCB_NORMAL_ACTIVE`, `ASYNCB_BOOT_AUTOCONF`, `ASYNCB_CLOSING`,
`ASYNCB_CTS_FLOW`, `ASYNCB_CHECK_CD`, `ASYNCB_SHARE_IRQ`,
`ASYNCB_CONS_FLOW`, and `ASYNCB_FIRST_KERNEL` only under `#ifndef
__KERNEL__`.  Lines 84–95 likewise define `ASYNC_INITIALIZED`,
`ASYNC_NORMAL_ACTIVE`, `ASYNC_BOOT_AUTOCONF`, `ASYNC_CLOSING`,
`ASYNC_CTS_FLOW`, `ASYNC_CHECK_CD`, `ASYNC_SHARE_IRQ`, `ASYNC_CONS_FLOW`,
and `ASYNC_INTERNAL_FLAGS` only under that guard.  Therefore none of these 19
names exists in the original kernel translation unit after its required
`__KERNEL__` preprocessing.

The candidate declares all of those names as unconditional `pub const`s
(lines 36–45 and 78–86).  Its comment acknowledges the kernel exclusion but
then intentionally changes it.  This is not an allowlisted branding change
and violates the selected conditional inventory (`ifndef@42`, `endif@53`,
`ifndef@84`, and `endif@95` in `rewrite/SYMBOLS.tsv`).  Preserve the
kernel/userspace visibility split in the Rust build surface; do not make these
obsolete UAPI-only names available to the kernel configuration.

## Verified parity outside P1

- SPDX identifier and immutable provenance name the correct source, revision,
  common architecture membership, and task ID.
- The 18 userspace-visible `ASYNCB_*` bit positions at upstream lines 14–34
  are present with their exact integer values and an `int`-width Rust type.
- The 18 flag macros at lines 56–73 preserve all bit positions and the C
  `1U` width as `u32`; derived masks at lines 75–82 preserve their operand
  sets and resulting values.
- No types, storage, functions, ABI layouts, or branding deltas occur in this
  header beyond the macros above.

## Verdict

Reject pending correction of P1.  No source was edited and no build, formatter,
test, or runtime command was run.
