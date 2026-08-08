# Rust review — S016368

Reviewed 2026-08-08T08:02:45.819Z as independent Rust-review slot 2 for
P01 attempt 2.

## Verdict: REJECT

### R1 — `issecure_mask` loses the UAPI macro interface and its C integer semantics (high)

`include/uapi/linux/securebits.h:9` defines the function-like macro
`issecure_mask(X)` for every includer. The candidate at
`src/include/uapi/linux/securebits.rs:13-17` defines an unexported,
module-lexical `macro_rules!` macro and consumes it only while constructing the
constants in this file. A sibling translation of
`include/linux/securebits.h` cannot invoke that macro to express its
`issecure(X)` macro, and the pinned Linux users include direct uses in
`security/commoncap.c:994,1394,1396`. The generated Rust source therefore
does not provide the selected source-level macro mechanism to its consumers.

The replacement also has no fixed C `int` result type at external call sites:
the unsuffixed Rust `1` is inferred from the expansion context, whereas the C
shift expression has an `int` left operand and hence an `int` result after the
C integer promotions. For the six local constant arguments (0 through 10),
the explicit `i32` constant targets happen to preserve the numerical values,
but that does not make the macro replacement equivalent for the selected
header's consumers. An applier must restore an externally usable, explicitly
typed mapping whose evaluation, shift and invalid-shift behavior is established
from the pinned source and its selected call contexts; do not silently replace
it with a differently typed convenience helper.

## Source checks with no finding

- The exact provenance header names `include/uapi/linux/securebits.h`, task
  `S016368`, `common`, and Linux revision
  `425f94c2954b1fe80ebdbf9b29854e89750355df`, matching `vendor/linux.SHA`.
- Every fixed object-like securebit macro in lines 11-81 of the complete pinned
  header is represented by a public `i32` constant. Their shifts are only
  0..10, so the constants retain the C `int` values; `SECURE_ALL_BITS` is
  `0x555`, `SECURE_ALL_LOCKS` is `0xaaa`, and
  `SECURE_ALL_UNPRIVILEGED` is `0x500`.
- This header declares no data layout, FFI symbol, pointer, borrow, pinning,
  interior-mutability, `Send`/`Sync`, callback, refcount, RCU, allocation, or
  unsafe boundary. The candidate contains no `unsafe`, `Drop`, allocation,
  panic helper, or bounds-sensitive operation. No additional Rust-specific
  finding arises in those categories.

## Attestation and snapshot

- Scope/task record: `include/uapi/linux/securebits.h` ->
  `src/include/uapi/linux/securebits.rs`; class `RUST_TRANSLATE`;
  architectures `common`; queue state at review: `REVIEWING` under P01,
  attempt 2.
- Complete pinned source inspected: `vendor/linux/include/uapi/linux/securebits.h`
  (lines 1-83), plus its macro consumer context in
  `vendor/linux/include/linux/securebits.h` and
  `vendor/linux/security/commoncap.c`.
- Current hashes at review:
  - `vendor/linux.SHA`: `7d3ae3944cd4d7a7d27b0df137485334e72bc9b9e04657abec78c4249ac9f692`
  - `vendor/linux/include/uapi/linux/securebits.h`: `fb1b1aa8f7fd6345f38cf1e6c062503b0fd09fd783d7f3155b0980196e542dfc`
  - `src/include/uapi/linux/securebits.rs`: `a93aa8de7027a7be4d1446565bb6f8538184dd838f3f49542e260dba29fb2d41`
  - `rewrite/TRANSLATION_TASKS.sha256`: `4f1888430dce939d6ee56d13a74cdbba740527ae89c05ec66aaefa0733c9a25d`
  - `rewrite/PHASE0_IDENTITY.tsv`: `0123af9e96b58af4e98b91a2de5608d36b63c67864b9041d3340587f3dbe40d2`

This was a manual source-only review. No compiler, formatter, linker,
rust-analyzer diagnostic, test, executable, archive/history, implementation
rationale, parity report, or Git evidence was read or used. The unresolved R1
requires applier disposition before this task can be accepted.
