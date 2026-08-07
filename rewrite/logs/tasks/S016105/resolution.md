# Applier resolution — S016105, attempt 2

Result: **BLOCKED**.  This is a source-only adjudication of
`vendor/linux/include/uapi/linux/dpll.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.  No compiler, formatter, build,
test, diagnostic, or historical translation was used.

## P1 / RUST-1 — accepted; exact correction identified, but no acceptance

`dpll.h:1` is exactly
`SPDX-License-Identifier: ((GPL-2.0 WITH Linux-syscall-note) OR BSD-3-Clause)`.
The destination's `GPL-2.0-only` is not source-faithful.  The source-backed
correction is to use the exact upstream expression.  It is not applied to
turn this blocked, reviewed candidate into a different unreviewed candidate;
the task must be reimplemented and independently reviewed after the blockers
below are resolved.

## P2 — accepted; no faithful Rust-only equivalent established

`dpll.h:7-8` uses `#ifndef _UAPI_LINUX_DPLL_H` followed by `#define
_UAPI_LINUX_DPLL_H`, and `dpll.h:310` closes that preprocessor conditional.
This is a C preprocessing contract, not a Rust value/type declaration.  The
pinned source and frozen records provide no Rust-side mechanism that preserves
both the macro name and conditional one-inclusion behavior for original C
consumers.  Adding a Rust item of that name would not define a C preprocessor
macro; treating a module as equivalent would not preserve the macro interface.
The proposal records that prematurely mark these guard records complete cannot
be accepted.

## RUST-2 — accepted; enum ABI and raw domain remain unresolved

The source declares distinct C enum types at `dpll.h:20,43,72,91,114,133,151,
173,194,212,227,232,252,290`.  The pinned core interface uses these types in
value and pointer callback positions, e.g. `include/linux/dpll.h:24-58` and
`:75-131`.  The frozen `ABI.tsv` rows for every one of those enum types on
both `aarch64` and `x86_64` remain `PENDING_REVIEW`; they do not establish the
C representation, calling-boundary treatment, or behavior for an out-of-range
C-originated value.  `pub type ... = u32` therefore fixes an unsigned raw
representation and erases the distinct C type without source authority.
Neither a Rust fieldless enum nor a newtype is proven faithful from the allowed
evidence.  This blocks final semantic closure.

## RUST-3 — accepted; macro category semantics remain unresolved

The source has unsuffixed C integer macro replacements at `dpll.h:11,106,
160-163,218-219` and C string-literal macro replacements at `dpll.h:10,308`.
The candidate changes these into `u32` constants and Rust byte-slice reference
constants.  The frozen records do not establish that those substitutions
preserve the C integer expression types or the string-literal array-lvalue/
pointer-decay behavior at an original-driver boundary.  A Rust `const` or
`static` cannot itself be the same C macro replacement.  No exact translation
is established from the permitted source evidence.

## Final disposition

The candidate's ordinal values and byte payloads were not disputed, but those
facts do not resolve the C preprocessing and ABI contracts above.  No final
semantic completion attestation is truthful, and no frozen semantic record is
closed by this resolution.  The queue was transitioned to `BLOCKED` with the
same three concrete unresolved contracts.  The destination is intentionally
unchanged: a partial SPDX-only correction would leave the task unacceptably
incomplete and would invalidate the reviewed candidate binding without making
it source-faithful as a whole.
