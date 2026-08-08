# Parity review — S016368 / P01 / slot 1

## Verdict

FINDINGS: `SC1-001`

## Materials inspected

- Pinned source: `vendor/linux/include/uapi/linux/securebits.h` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Candidate and task diff: `src/include/uapi/linux/securebits.rs` and
  `rewrite/logs/tasks/S016368/candidate.diff`.
- Frozen task/scope/symbol/map records for S016368, the empty branding
  allowlist, and the immediate pinned wrapper/call sites:
  `include/linux/securebits.h` and `security/commoncap.c`.

## Finding

### SC1-001 — `issecure_mask` no longer has the Linux macro operand contract

Linux symbol: `issecure_mask`.

Affected frozen semantic record keys:
`SC1-92239f10b9eaa3d897af9aca79705572e5216edf5c36e59ddc9ffccf47954e8d`,
`SC1-717a73bb9bd61b7dc369dc90051c18e79670c774d777270be83d1e2f538c0088`,
`SC1-d5032262e96fcf2fe00cabca2035a1664cbee64759104c8136f699a769fb1a79`,
and
`SC1-20a010570719de1a4ebfce6715bb0bbfdc89d3a28c5a6336dffe33070f9fb74c`.

Pinned local evidence: line 9 of
`vendor/linux/include/uapi/linux/securebits.h` defines
`#define issecure_mask(X) (1 << (X))`.  It is an expression-like macro: its
operand is evaluated once after C integer promotions, and the left operand is
the C `int` literal `1`.  The local wrapper at
`vendor/linux/include/linux/securebits.h:7` embeds it in `issecure(X)`, and
the pinned `security/commoncap.c` directly uses it for bitwise updates (for
example lines 994, 1394, and 1396).  The frozen `SYMBOLS.tsv` lists
`issecure_mask` as an operative macro for both approved architectures.

Candidate evidence: `src/include/uapi/linux/securebits.rs:14-16` replaces the
macro with `pub const fn issecure_mask(x: u32) -> i32 { 1_i32 << x }`, while
all in-file calls add `as u32` casts.

This is not a source-equivalent replacement for the exported macro contract:
callers must now supply `u32` (or perform a cast), instead of any C integer
expression subject to the original integer promotions.  It also gives
out-of-range and negative operands Rust function/shift semantics rather than
the original C expression's `int` shift semantics.  The candidate happens to
produce the correct values for this header's six fixed non-negative bit
indices, but it does not preserve the operative `issecure_mask(X)` interface
or its general evaluation/type behavior.  Replace it with a mapping whose
accepted operand and C-`int` shift semantics are explicitly preserved, or
block the task if that cannot be expressed exactly in the surrounding Rust
ABI/module design.

## Checked without additional finding

- The SPDX identifier and all four immutable provenance lines match the task,
  pinned SHA, source path, and `common` architecture membership.
- `SECUREBITS_DEFAULT`, the twelve bit-index constants, all twelve `SECBIT_*`
  constants, `SECURE_ALL_BITS`, `SECURE_ALL_LOCKS`, and
  `SECURE_ALL_UNPRIVILEGED` have the same fixed signed-`int` values and the
  same `|`/`<<` grouping for their defined fixed operands.
- The source guard `_UAPI_LINUX_SECUREBITS_H` has no extra candidate-visible
  runtime or linkage artifact; module inclusion/guard realization remains a
  surrounding module-system concern rather than a second header-body finding.
- No Linux-to-Lupos name delta is present; the frozen branding allowlist has
  only its header.

No compiler, formatter, linker, test, rust-analyzer diagnostic, or runtime
tool was invoked or used as evidence.
