# S016368 Rust source review (slot 2)

Task: `S016368`  
Pipeline/attempt: `P01` / `1`  
Reviewed source: `vendor/linux/include/uapi/linux/securebits.h`  
Candidate: `src/include/uapi/linux/securebits.rs`

## Frozen evidence

- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Phase 0 identity: `0123af9e96b58af4e98b91a2de5608d36b63c67864b9041d3340587f3dbe40d2`; binding digest `03f3c4afb3c7edc167ddeadac5493cbee736042cb7781182d4fdf43b2b79166d`.
- Queue fingerprint: `cfa8dcf0d81c408de38d4202fe1ce9d9e5e301d1e67439c9530da94b1e64ee3f`.
- Scope row is `RUST_TRANSLATE`, `common`, destination `src/include/uapi/linux/securebits.rs`; its selected symbols cover the include guard and every macro for both frozen architectures.

## Findings

### SC1 — `issecure_mask` is not a semantics-preserving mapping of the C function-like macro (must fix)

Linux defines `issecure_mask(X)` as the parenthesized expression `(1 << (X))` at `include/uapi/linux/securebits.h:9`.  Its left operand and result have C `int` type on both frozen targets; its right operand is an arbitrary integer expression subject to C's integer-promotion rules, and it is evaluated in the caller's expression context exactly once.

The candidate replaces that macro with `pub const fn issecure_mask(x: u32) -> i32` at Rust lines 13--15.  This narrows the accepted right-operand type from the C macro's integer-expression contract to `u32`.  The translated secure-bit indexes are themselves `i32`, so every internal call has already had to add an `as u32` conversion (for example lines 22--23); a direct faithful use such as the pinned `include/linux/securebits.h:7` wrapper's `issecure_mask(X)` no longer has the same expression/type contract.  A C `int`, signed integral expression, or an integral expression of a wider type is not represented without a caller-side, potentially lossy or sign-changing cast.

This is not an ownership or safety improvement: the source item is an operative UAPI macro and must retain its caller-context arithmetic/evaluation mechanism.  Represent it as a Rust macro (or another mapping proven to preserve the full selected call contract) rather than a `u32` function, and then recheck every caller.  The fixed constants happen to evaluate to the same `i32` values, but that does not repair the public macro mapping.

### SC2 — `candidate.diff` is not the current reviewed candidate snapshot (must resolve before application)

The current candidate has the six-line secure-setting explanatory comment at Rust lines 7--12, while `rewrite/logs/tasks/S016368/candidate.diff` begins the added body immediately with `pub const fn issecure_mask` and contains none of those lines.  The task is already `REVIEWING`, so the supplied snapshot does not identify the exact source inspected by this reviewer.  Regenerate/replace the task-local candidate snapshot from the current candidate and re-run any review whose input was the stale snapshot; otherwise the evidence cannot establish which candidate the two reviewers approved.

## Manual Rust-semantics audit

- No `unsafe`, raw pointers, references, pinning, interior mutability, atomics, `Send`/`Sync` implementation, callbacks, allocation, FFI, layout attribute, `Drop`, panic helper, or bounds-indexing operation occurs in the candidate.  Therefore there is no unsafe block or ownership/aliasing/lifetime boundary to approve.
- All fixed source macros and aggregate masks are present with `i32` values.  For their fixed indices `0` through `11`, the candidate's calculated values and the C `int` expressions agree; `SECURE_ALL_BITS`, `SECURE_ALL_LOCKS`, and `SECURE_ALL_UNPRIVILEGED` retain their source operation order and fit in signed 32-bit range.  This limited value agreement does not discharge SC1.
- The C include guard is a preprocessor multiple-inclusion mechanism.  The Rust module item is ordinarily instantiated once by its module declaration; no separate C preprocessor guard state is represented.  No source-level ABI object, linkage name, packing, endian conversion, or calling convention is defined by this header.
- Provenance fields, SPDX identifier, source path, revision, task ID, and `common` architecture designation agree with the frozen task/source records.  `common` is the queue's architecture field and denotes the x86_64/aarch64 union.

## Review result

REJECTED_PENDING_APPLICATION: SC1 changes an operative macro's type and caller-expression semantics.  SC2 independently invalidates the candidate-review evidence binding.  No compiler, formatter, test, runtime, or rust-analyzer diagnostic was invoked or used.
