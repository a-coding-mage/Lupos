# Resolution — S016002 / P02 / attempt 1

Reviewed manually against the complete pinned
`vendor/linux/include/uapi/asm-generic/errno-base.h`, its direct pinned UAPI
consumer `vendor/linux/include/uapi/asm-generic/errno.h`, the sealed candidate
at `src/include/uapi/asm-generic/errno-base.rs`, the candidate artifact, both
review reports and attestations, and the task-local frozen symbol, ABI, and
lifetime records.  No compiler, formatter, linker, test, runtime tool,
analyzer, or historical Lupos source was used.

## P1 — selected C macro and include-guard interface

**Disposition: accepted; unresolved — recommend `BLOCKED`.**

The pinned source defines `_ASM_GENERIC_ERRNO_BASE_H` with `#ifndef`/`#define`
and defines `EPERM` through `ERANGE` as object-like C preprocessor macros
(lines 2--38).  Its direct UAPI consumer, `include/uapi/asm-generic/errno.h:5`,
consumes the header with `#include <asm-generic/errno-base.h>` and continues
in the same C macro namespace, including `#define EWOULDBLOCK EAGAIN` at line
29.  The sealed Rust candidate instead publishes Rust `i32` constants and a
comment that a Rust module corresponds to the guard.  The constants establish
the literal values, but they do not establish C preprocessing, include-order,
or macro-redefinition behavior.

The frozen `SYMBOLS.tsv` records the guard and all errno macros for both
architectures as `PENDING_REVIEW`.  The task has no frozen ABI or lifetime
record that defines a C-facing macro export, generated binding, or header
generation bridge.  The Rust review correctly establishes the literal
`i32` values and absence of Rust ownership/layout hazards, but it cannot
establish the missing selected C interface.  Supplying a new bridge or
declaring module constants equivalent would introduce an unreviewed contract
without pinned-source or frozen-manifest support.

## P1 — candidate artifact does not bind the reviewed source

**Disposition: accepted; unresolved — recommend `BLOCKED`.**

`rewrite/logs/tasks/S016002/candidate.diff` is a 278-byte prose assertion,
not a focused source snapshot or diff.  It contains no candidate lines or
source hash, so it does not bind the reviewed candidate to the implementation
transition.  The sealed semantic proposal records that artifact's hash, which
does not cure the absence of source binding.  The sealed candidate must not
be edited at this stage, and replacing the artifact now would invalidate the
completed review evidence.

No source edit is justified.  The task must be `BLOCKED` until authoritative
frozen evidence specifies a parity-preserving C macro/header bridge and a
fresh candidate/evidence cycle can bind a concrete source revision.
