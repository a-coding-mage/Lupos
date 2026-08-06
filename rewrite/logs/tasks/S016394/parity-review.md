# Parity review — S016394 (slot 1)

## Result

ACCEPT — no parity findings.

## Evidence reviewed

- Pinned oracle: `vendor/linux/include/uapi/linux/sunrpc/debug.h`, revision
  `425f94c2954b1fe80ebdbf9b29854e89750355df`, complete lines 1–49.
- Candidate: `src/include/uapi/linux/sunrpc/debug.rs`, complete lines 1–38.
- Frozen task row `S016394` (`common`, `RUST_TRANSLATE`), scope/header-closure
  evidence, `FILE_MAP.tsv`, `SYMBOLS.tsv`, `ABI.tsv`, and `LIFETIMES.tsv`.
- The wrapper `include/linux/sunrpc/debug.h` and selected consumer contexts,
  including the sunrpc debug-facility definitions and the lockd/nfsd includes.

## Exhaustive comparison

| Oracle surface | Candidate surface | Verdict |
| --- | --- | --- |
| 13 `RPCDBG_*` object-like macros (`XPRT` through `ALL`) | 13 same-named public `i32` constants, in source order | Exact names and values: `0x0001` through `0x0800`, plus `RPCDBG_ALL = 0x7fff`. |
| anonymous enum's eight enumerators | 8 same-named public `i32` constants | Exact C `int` category and values preserved: explicit `CTL_RPCDEBUG = 1`, followed by implicit values 2–8 through `CTL_MAX_RESVPORT`. The anonymous enum creates no named C type or object to reproduce. |
| Header conditionals | None required in Rust | The only conditional is the ordinary include guard; the selected source has no configuration-controlled branch. |
| UAPI public names used by the wrapper/selected consumers | Public constants | `RPCDBG_*` names remain available for the facility masks; no renamed/extra/replaced interface was found. |
| SPDX, upstream attribution, immutable provenance | Present | SPDX expression, source path, frozen revision, `common` architecture set, and task ID match the task and pinned source. |

The masks are all representable as C `int` on both frozen targets; their explicit
Rust `i32` category preserves the source integer category and every bit/value.
There are no functions, data objects, layouts, linkage requirements, ownership,
locking, allocation, error, cleanup, or configuration semantics in this header.

## Findings

None.
