# Resolution — S016011, attempt 4

## Applier determination

The current candidate is source-complete for the frozen common task.  No source
change is required or made by this application stage.

The pinned oracle is
`vendor/linux/include/uapi/asm-generic/mman-common.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, which agrees with
`vendor/linux.SHA`, the frozen S016011 queue row, and the candidate provenance.
Its SPDX expression is exactly `GPL-2.0 WITH Linux-syscall-note`; the candidate
preserves that expression, the common architecture membership, source path,
revision, and task ID.

Direct source comparison gives 53 non-guard `#define` names in the pinned
header and 53 `pub const` names in the candidate, with neither a missing nor an
extra name.  Every source value is an unsuffixed integer literal at or below
`0x04000000`, hence representable by the 32-bit `int` used by both frozen Linux
targets; the corresponding `i32` constants preserve each value and signed
width.  The candidate retains `PKEY_ACCESS_MASK` as
`PKEY_DISABLE_ACCESS | PKEY_DISABLE_WRITE`, matching the expression at pinned
lines 88--91 rather than substituting a literal.

The only conditional in the pinned header is the C inclusion guard at lines
2--3 and 94.  It has duplicate-definition suppression semantics, not a UAPI
data/symbol ABI.  The path-scoped Rust module is the exact mapping for that
once-only role and does not expose a replacement guard symbol.  The narrow
contexts confirm the boundary: `include/uapi/asm-generic/mman.h` includes this
header; `arch/x86/include/uapi/asm/mman.h` then inherits that generic result;
and `arch/arm64/include/uapi/asm/mman.h` subsequently undefines and redefines
only `PKEY_ACCESS_MASK`.  That AArch64-specific override belongs to its own
source task and is not a defect in this common-header translation.

There are no functions, objects, layouts, linkage, allocation, locking,
ownership, lifetime, RCU/refcount, error, cleanup, or configuration branches
beyond that guard in the oracle.  The task has no S016011 rows in the frozen
ABI or lifetime ledgers.  The current semantic-closure proposal covers the
scope status, both architecture copies of the guard and each selected macro;
both review attestations approve it.

## Review-finding dispositions

| Review report | Finding | Disposition | Source-backed resolution |
| --- | --- | --- | --- |
| `parity-review.md` | No finding (APPROVE) | Confirmed; no source change | Reopened the complete pinned header and verified exact SPDX/provenance, all 53 non-guard names and values, the computed `PKEY_ACCESS_MASK`, and the guard-to-module mapping. |
| `rust-review.md` | No finding (APPROVE) | Confirmed; no source change | The values fit signed 32-bit `int` on the two frozen targets; this file has no unsafe, FFI/layout, pointer, ownership, panic, or evaluation-order hazard. The AArch64 later override is expressly outside the common header. |

## DONE eligibility

Eligible for `DONE` by the coordinator after its atomic queue and
semantic-closure application checks.  The required five evidence files are
non-empty.  The task-local closure schema defines `candidate_sha256` over the
candidate snapshot, `candidate.diff`, not over the destination Rust file:
the proposal's `candidate_sha256`
`697a537a1212e3dd097d00affdd93356b80d739844e7cb94641569b19bb8a895`
equals the current `candidate.diff` digest, and its `implementation_sha256`
`84f6fde03ff08b3f41eaf3eef2644d1aab7ab0ca9af2f3383858acfa24701e00`
equals the current `implementation.md` digest.  Both independent
semantic-review attestations approve the same proposal digest,
`9bffef8baad96fad92d1f4bf8ac84b32a664d137609f790acce649b9e7a9182a`.

The source-file digest is not a semantic-closure binding and supplies no
contrary evidence.  The proposal's final values close the S016011 scope and
symbol semantic records, including both architecture copies of the guard and
all selected macros.  No source-backed blocker remains.  This determination is
source-only; no compilation, formatting, linking, runtime, test, benchmark, or
other toolchain evidence was used.
