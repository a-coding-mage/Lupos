# Parity review — S016151

Reviewed `vendor/linux/include/uapi/linux/hw_breakpoint.h` at pinned revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/uapi/linux/hw_breakpoint.rs`.

## Result

No parity findings.

## Evidence checked

- The candidate preserves the UAPI SPDX expression and immutable source,
  revision, architecture, and task provenance (candidate lines 1–5; upstream
  line 1).
- The first anonymous C enum's eight public enumerators are represented as
  public `i32` constants with the exact explicit values 1 through 8 (upstream
  lines 5–14; candidate lines 8–15). C anonymous-enum enumerators have `int`
  type; `i32` retains that signed 32-bit public value type for this Linux UAPI.
- The second anonymous C enum's public constants preserve their names and
  values: `EMPTY=0`, `R=1`, `W=2`, and `X=4` (upstream lines 16–23; candidate
  lines 18–23). The derived expressions remain expressions rather than
  substituted literals: `RW = R | W` and `INVALID = RW | X` (upstream lines
  20 and 22; candidate lines 21 and 23), yielding 3 and 7 respectively.
- The source has only its ordinary UAPI include guard (upstream lines 2–3 and
  25), which has no Rust configuration analogue; the candidate exports the
  same constants from its path-preserving Rust module. There are no
  configuration-selected branches in this header. `SCOPE.tsv` records the
  task as `common` for both frozen x86_64 and aarch64 configurations.
- Relevant pinned consumers include the UAPI through
  `include/linux/hw_breakpoint.h` (line 6). Their comparisons, switch cases,
  and bitwise tests use exactly the enumerator names and values preserved by
  the candidate: generic validation rejects `EMPTY` and `INVALID`, and the
  x86_64/aarch64 breakpoint paths use the length and access constants.

This was a manual, source-only review; no compiler, formatter, linker, test,
or runtime tool was invoked.
