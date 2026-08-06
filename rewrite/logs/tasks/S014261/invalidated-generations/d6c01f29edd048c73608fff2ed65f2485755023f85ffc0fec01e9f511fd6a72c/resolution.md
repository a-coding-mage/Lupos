# Applier resolution — S014261

## Inputs reopened

- Pinned source: `vendor/linux/include/linux/lsm/smack.h` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df`; the checked-out vendor tree
  equals `vendor/linux.SHA`.
- Candidate: `src/include/linux/lsm/smack.rs`.
- Frozen configurations:
  `rewrite/configs/x86_64/frozen.config:4748` and
  `rewrite/configs/aarch64/frozen.config:11201`, each of which records
  `# CONFIG_SECURITY_SMACK is not set`.
- Context: `vendor/linux/include/linux/security.h:164-169` embeds this named
  type as the `smack` member of `struct lsm_prop`; the header initializes and
  compares that containing aggregate by its full selected representation at
  lines 302-320.
- Evidence reopened: `implementation.md`, `candidate.diff`,
  `parity-review.md`, and `rust-review.md`.

No compiler, formatter, analyzer, build, test, debugger, or historical Lupos
source was used.

## Disposition

Accepted without a semantic candidate change. The applier corrected the Rust
SPDX identifier to the required immutable provenance form
`GPL-2.0-only`; no operative Rust definition changed.

The complete upstream header has one selected type. Its sole possible member,
`struct smack_known *skp`, is entirely inside `#ifdef
CONFIG_SECURITY_SMACK`. Since that configuration symbol is absent in both
frozen configurations, preprocessing selects a memberless
`struct lsm_prop_smack` on x86_64 and AArch64. The forward declaration of
`struct smack_known` has no selected consumer in this header. The candidate's
public `#[repr(C)] pub struct lsm_prop_smack {}` preserves the named,
memberless aggregate used for the `smack` field of `struct lsm_prop`, without
introducing disabled pointer storage or a stronger Rust ownership contract.

`#[repr(C)]` is required because the type is an embedded Linux aggregate. It
has no fields, no functions, no static storage, no exported symbol, no calling
convention, no allocation, no locking, no RCU/refcount protocol, and no
cleanup/drop action in the selected configuration union. The candidate has no
`unsafe` code, references, raw pointers, or `Drop`; consequently there is no
Rust aliasing, lifetime, provenance, synchronization, or panic behavior to
adjudicate for this task.

## Review finding dispositions

- Parity review: accepted with no finding. Reopened source and frozen
  configuration evidence confirm that the guarded `skp` member is not selected
  and that no unselected Smack declaration or behavior was imported.
- Rust review: accepted with no finding. `#[repr(C)]` plus a zero-field named
  type is the required Rust representation of this selected memberless
  aggregate; no unsafe or ownership mechanism is present.

## Closed Phase-0 semantic records

The following `PENDING_REVIEW` records for S014261 are resolved for both
`x86_64` and `aarch64` by the evidence above:

- `ifndef@6`, `__LINUX_LSM_SMACK_H`, and both closing conditionals are
  preprocessing-only include-guard mechanics with no separate Rust object,
  ABI, or runtime behavior.
- `ifdef@12 CONFIG_SECURITY_SMACK` resolves false in both frozen configs; its
  guarded `skp` declaration is absent from the selected source.
- `struct lsm_prop_smack` is a named C-representation aggregate embedded by
  value in `struct lsm_prop`; it is memberless in the selected union and is
  represented by the public zero-field `#[repr(C)]` Rust type.
- Ownership, lifetime, locking, RCU, refcounting, allocation, and cleanup are
  `NOT_APPLICABLE`: no selected field remains and the header defines no
  operations.
- ABI intent is internal type layout only: there is no linkage, export,
  function ABI, alignment/packing override, or symbol to preserve beyond the
  named `#[repr(C)]` aggregate at its containing-field position.

No unresolved source, scope, ABI, lifetime, or branding question remains for
this task.
