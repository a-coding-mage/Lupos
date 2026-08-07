# Rust source review — S016189 / attempt 1 / P01

Verdict: **REJECT — source and closure evidence are not acceptable for slot 2.**

Reviewed only the current candidate, pinned `include/uapi/linux/input-event-codes.h`,
the S016189 current semantic proposal/seal, frozen scope/config identity records, and
the queue row. No compiler, formatter, test, rust-analyzer diagnostic, or historical
Lupos source was used.

Pinned context: Linux `425f94c2954b1fe80ebdbf9b29854e89750355df`; task is
`RUST_TRANSLATE`, `common`, with frozen x86_64 and aarch64 configurations
(`rewrite/SCOPE.tsv:16190`, configuration metadata).  The proposal seal is
`62e1c3464bcd646dd6fcbf2147e496d4cc2246d2c4973923159a0466349a6121`, Phase-0
identity is `0123af9e96b58af4e98b91a2de5608d36b63c67864b9041d3340587f3dbe40d2`,
and the queue fingerprint bound by that seal is
`cfa8dcf0d81c408de38d4202fe1ce9d9e5e301d1e67439c9530da94b1e64ee3f`.

## Findings

1. **RUST-S016189-001 — current candidate invalidates the sealed semantic proposal (blocking).**
   The sealed proposal binds candidate SHA-256
   `9937906354485fd12dbeef6173d96d49e1d544e951445d366311ed3125980181`; the
   current candidate hashes to
   `de3aa54b04af8f418244bb208ab99eeb51c73f8ae91de80b83e446ae6a94a90c`.
   `semantic_closure.py` requires a sealed proposal to validate the current candidate
   before creating an attestation. Therefore none of its 3,189 proposed semantic
   closures can be attested for this source revision, and slot 2 must not be recorded.
   Closure evidence: proposal seal above; every proposal record carries the stale
   candidate binding, including the task-level scope closure
   `SC1-322f4ed4b0d2eec28b0a86e8e7f15c30972b4c0049db89b4c3460ee31469864b`.

2. **RUST-S016189-002 — include guard was converted into an exported numeric UAPI item.**
   Upstream uses an empty preprocessor guard, `#ifndef` / `#define
   _UAPI_INPUT_EVENT_CODES_H` at `vendor/linux/include/uapi/linux/input-event-codes.h:16-17`,
   and closes it at line 1016. The candidate instead makes
   `_UAPI_INPUT_EVENT_CODES_H` a public `u32` value of `1`
   (`src/include/uapi/linux/input-event-codes.rs:22-23`). This adds a Rust item with
   numeric value and type where Linux exposes only preprocessing state; it is not a
   selected numeric UAPI code and alters textual-include semantics. Closure keys:
   `SC1-ad49236e37be6803109f421ace115e9360f2dde28cd045f1a1541e2a580d30b4`,
   `SC1-18c152c27227d42c2a24a2a84bf995c0183e845556fb851ae7269a4cbb3605f5`,
   `SC1-c6d5f1cd2d75d64ab63d2af3482c2d7ba280f8f89fe3353386db431d0efd1908`, and
   `SC1-8eb6146ac527897d37af55f236126ac2fb88d3e51c7f9270a0875ad9b17c3235`.

3. **RUST-S016189-003 — all code macros were forced to `u32`, changing C integer semantics.**
   The 795 upstream object-like code macros are unsuffixed C integer constants or
   aliases; examples include `INPUT_PROP_CNT` at upstream line 33 and `KEY_CNT` at
   line 838. On both frozen targets those literals and the `+ 1` expressions have the
   C `int` type unless a consuming expression applies its own conversions. The
   candidate gives all 796 `pub const` items (including the spurious guard) type
   `u32`, e.g. `INPUT_PROP_*` at candidate lines 29-38 and `KEY_CNT` at line 844.
   That changes signedness, promotion and mixed-expression behavior, and prevents
   the original context-dependent C macro type from being preserved. It is especially
   material for each `*_CNT` expression and any caller using a signed index or
   comparison. The rewrite needs a source-derived, use-site-compatible representation
   rather than a blanket unsigned type. Closure evidence: the 1,592
   `selection_expression` records and 1,596 `status` records in the current proposal.
   Representative closure keys for `INPUT_PROP_CNT` and `KEY_CNT` are
   `SC1-30912b71abba72e9777c04cbae567d8b0b3ce9bb6bd13f85e74bfc0275748059`,
   `SC1-11f1639b0311909c2c4232c2a54dcc188b403df4e8a793a0843dc14ecb7eb5ff`,
   `SC1-72f71192344995c2277cef5e03dd5e751530aad12f006429fbab394845f93d26`,
   `SC1-41641b737656b374e0154affc71b40c0629ce741619cb3114fd3f842eb1cc7f1`,
   `SC1-ebb1d5d14535ba6bdd9f8f8d7a28c73446c7f56658bdfb78f342d12fcd18c78c`,
   `SC1-7ae61d4056cdc8a1e8eaf8e4d1c96e1b549de265141a1f937b625d32c45e84a2`,
   `SC1-6071b08fab790d63d17de18b67fcc649b802f3a7cad4c1869ebcf25c35ded390`, and
   `SC1-5b98ef92870b5e424c2fce3aaeb5a465cb9b1b69f05b525b8d437c4bcdf64dc1`.

## Completed source-only checks

- Read the complete pinned header (lines 1-1016) and current candidate (lines
  1-1024). Line-defined source macro/constant comparison found 795 upstream object
  macros and 796 candidate constants; the sole name-set delta is the fabricated guard.
  Numeric spellings and alias right-hand sides otherwise match their corresponding
  source definitions.
- The pinned header has no configuration branch beyond its include guard. The candidate
  has no `#[cfg]`, FFI declaration, layout type, `unsafe`, interior mutability,
  callback, allocation, `Drop`, panic, `todo!`, `unimplemented!`, or Rust test code.
  Those categories introduce no additional finding in this constants-only file.
- No `repr(C)` layout or calling convention is present to review; the defects above are
  Rust item/type and closure-binding defects, not a layout issue.

Because finding 1 makes the proposed seal stale, I intentionally did **not** submit
`semantic_closure.py review --slot 2` and did **not** invoke `rewrite_queue.py
mark-review`. Doing either would fail its current-source binding requirement or would
misrepresent the reviewed revision. No queue mutation was made.
