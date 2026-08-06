# Applier resolution — S016394

## Inputs independently reopened

- Pinned oracle: `vendor/linux/include/uapi/linux/sunrpc/debug.h`, complete
  lines 1–49, at `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Fresh candidate: `src/include/uapi/linux/sunrpc/debug.rs`, complete lines
  1–38.
- Frozen common task row, the two frozen configurations, header-closure
  evidence, and the relevant wrapper `include/linux/sunrpc/debug.h`.

## Review dispositions

1. Parity review: accepted. Rechecked the thirteen `RPCDBG_*` object-like
   macros on source lines 16–28. Each unsuffixed literal is representable as
   signed C `int` on both frozen targets and is exposed under the same public
   name as an `i32` constant; `RPCDBG_ALL` remains exactly `0x7fff`.
2. Parity review: accepted. Rechecked the anonymous enum on lines 38–47. It
   declares neither a tag nor an object; its enumerators have C `int` category,
   with explicit `CTL_RPCDEBUG = 1` and seven implicit increments through
   `CTL_MAX_RESVPORT = 8`. The eight public `i32` constants preserve those
   names and values.
3. Rust review: accepted. There is no layout-bearing object, linkage,
   allocation, ownership, locking, cleanup, unsafe operation, or executable
   control flow in this header. The include guard at lines 10–11 and 49 is a
   source-inclusion mechanism with no Rust runtime/item counterpart.
4. Provenance and scope: accepted. The source path, pinned revision, `common`
   architecture membership, UAPI SPDX expression, attribution, and task ID
   match the frozen task. No branding delta or test/stub was introduced.

## Task-record closure

All S016394 `PENDING_REVIEW` records are now `COMPLETE` in `SYMBOLS.tsv`,
`ABI.tsv`, and `LIFETIMES.tsv`. The records cite the exact source lines and
the frozen x86_64/aarch64 target compile-command metadata. They record the
ordinary include guard as non-ABI, the unconditional masks as C-`int`/`i32`,
and the anonymous enum as non-object/non-owning while preserving its C-`int`
enumerator category. No unrelated task record was changed.

## Outcome

No source change was required after independent review. This task is source
translation pipeline complete only; it has not been compiled, linked, run, or
tested.
