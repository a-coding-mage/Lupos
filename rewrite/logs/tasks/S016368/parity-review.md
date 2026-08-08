# Parity review — S016368 / slot 1

Reviewed the current candidate and `candidate.diff` against the complete pinned
`vendor/linux/include/uapi/linux/securebits.h`, the frozen S016368 symbol
inventory for x86_64 and AArch64, the header-closure record, and direct local
consumers (`include/linux/securebits.h`, `security/commoncap.c`, and
`kernel/user_namespace.c`).  This was a manual source review only; no compiler,
formatter, test, or runtime tool was invoked.

## Finding P1 — `issecure_mask` has no established all-input C-to-Rust semantic mapping

- Linux symbol: `issecure_mask(X)`.
- Linux evidence: `vendor/linux/include/uapi/linux/securebits.h:9` defines the
  public UAPI macro exactly as `(1 << (X))`, with an unsuffixed C `int` left
  operand and a general macro parameter.  The frozen inventory records it as
  an operative macro for both architectures.  Its result feeds every
  `SECBIT_*` macro and `SECURE_ALL_BITS`, `SECURE_ALL_LOCKS`, and
  `SECURE_ALL_UNPRIVILEGED` at lines 22--81.
- Candidate evidence: `src/include/uapi/linux/securebits.rs:14-18` replaces it
  with `#[macro_export] macro_rules! issecure_mask { ($x:expr) =>
  (1_i32 << ($x)) }`.
- Parity issue: the candidate fixes the Rust operand and shift semantics but
  supplies no source-derived contract for every C-integer argument accepted by
  the public macro.  In particular, the pinned source itself does not limit
  `X` to a Rust expression type or establish how negative or width-or-greater
  counts must map from the C shift expression to Rust's checked/overflow-mode
  dependent shift behavior.  Such inputs therefore cannot be accepted as
  source-parity-preserved merely because every currently named securebit index
  is small.  No ABI or semantic record for S016368 resolves this conversion.
- Local caller evidence and bounded impact: the selected in-tree consumers use
  only fixed indices: `security/commoncap.c:994` uses
  `SECURE_KEEP_CAPS`, lines 1336--1360 consume the aggregate masks, and lines
  1394--1396 again use `SECURE_KEEP_CAPS`; `kernel/user_namespace.c:49` uses
  only `SECUREBITS_DEFAULT`.  Thus the candidate's `i32` values for the
  selected fixed indices and aggregate masks are arithmetically equal to the
  pinned header, but that does not settle the public parameterized macro.
- Required resolution: establish and record the exact supported argument
  domain and C-`int`/shift conversion contract from pinned local source and
  frozen ABI guidance, then make the macro and its Rust call provenance obey
  that contract.  If the source cannot establish a parity-preserving mapping
  for the public macro, keep the task blocked rather than assuming Rust shift
  behavior.

## Checked without additional finding

- All selected fixed-index macros are present with their pinned values:
  `SECUREBITS_DEFAULT`; the twelve `SECURE_*` index/lock constants; the twelve
  corresponding `SECBIT_*` masks; and the three aggregate masks.  For the
  source-defined indices 0 through 11, the candidate's `i32` calculations give
  the same `int` bit values used by the direct selected consumers.
- `SECURE_ALL_BITS` still selects bits 0, 2, 4, 6, 8, and 10; `SECURE_ALL_LOCKS`
  remains its one-bit-left shift; and `SECURE_ALL_UNPRIVILEGED` remains bits 8
  and 10.  No extra feature branch, state, allocation, linkage symbol, lock,
  refcount, or branding delta was introduced by this header-only candidate.
- Candidate provenance names the pinned source, pinned revision,
  `common` architecture set, and S016368; its SPDX identifier matches the
  pinned UAPI header.  No branding-allowlist exception applies.

Result: **not approved pending resolution of P1**.
