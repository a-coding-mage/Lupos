# S016327 applier resolution — attempt 1 / P02

## Disposition

**BLOCKED.** The sealed candidate cannot be accepted without guessing a Rust
representation and use-boundary contract that the pinned source and frozen
records do not establish.

## Evidence reopened

This source-only adjudication reopened the complete pinned
`vendor/linux/include/uapi/linux/personality.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the sealed candidate, proposal,
and both independent review reports. It also reopened the direct local context
in `vendor/linux/include/linux/personality.h`,
`vendor/linux/kernel/exec_domain.c:38-45`,
`vendor/linux/fs/exec.c:1595-1606`, `vendor/linux/kernel/sys.c:1325-1329`,
and `vendor/linux/arch/arm64/kernel/sys.c:31-36`.

The frozen records retain `PENDING_REVIEW` for every selected symbol expression
in this header. More specifically, `rewrite/ABI.tsv` has, for each target and
each of `anonymous_enum@11` and `anonymous_enum@42`, both
`layout=PENDING_REVIEW` and `alignment=PENDING_REVIEW`, with
`export_kind=PENDING_REVIEW`. The matching `rewrite/LIFETIMES.tsv` rows also
remain pending. The selected `PER_CLEAR_ON_SETID` macro rows in
`rewrite/SYMBOLS.tsv` are `PENDING_REVIEW` on both targets.

No compiler, formatter, analyzer, linker, test, runtime command, or historical
Rust source was used.

## Finding dispositions

### Rust F1 — anonymous C enumeration ABI and integer-domain contract

**Sustained; unresolved and blocking.** The two upstream anonymous enum
declarations at lines 11 and 42 establish the enumerator spellings and numeric
expressions, but no frozen, target-bound record establishes their chosen C
object representation, alignment, compatible integer type, or any explicit
Rust/C conversion boundary. The candidate changes all enumerators to fixed
`i32` constants.

The missing choice is material in source. `include/linux/personality.h:10`
forms `pers & PER_MASK`; `kernel/sys.c:1329` forms
`current->personality & UNAME26`; `arch/arm64/kernel/sys.c:31-36` receives an
`unsigned int` personality syscall argument and tests `PER_LINUX32`; and
`fs/exec.c:1595-1606` updates the unsigned `bprm->per_clear` with the macro.
The header itself gives no Rust-side type/conversion rule for these mixed C
integer operations. Selecting `i32`, `u32`, a transparent wrapper, or a
fieldless Rust enum would be an unreviewed design and would not close the
frozen ABI fields. The parity approval does not supply the missing
identity-bound ABI evidence, so it cannot disprove this finding.

### Rust F2 — `PER_CLEAR_ON_SETID` macro expression contract

**Sustained; unresolved and blocking.** Upstream lines 31-34 define a C
preprocessor replacement list comprising the four enum expressions
`READ_IMPLIES_EXEC`, `ADDR_NO_RANDOMIZE`, `ADDR_COMPAT_LAYOUT`, and
`MMAP_PAGE_ZERO`. Although all four literals produce the candidate's displayed
bit pattern and have no side effects, the frozen symbol record still leaves the
macro's selected expression pending on both architectures. At the direct
set-id call sites, `fs/exec.c:1600` and `:1605` OR the expansion into
`bprm->per_clear`; the exact translated expression type and conversion
boundary are therefore part of the operative contract.

Replacing that macro with an `i32` value proves only a value for the current
definition, not the source-proven Rust expression/use contract demanded by the
frozen records. There is no authorized local evidence selecting a faithful
alternative without altering the sealed candidate and invalidating both
reviews. No correction is made.

## Terminal conclusion

Both Rust-review findings remain open for the same missing frozen ABI and
mixed-integer expression evidence. The candidate, proposal, and review
artifacts remain sealed and unchanged. S016327 must be blocked rather than
marked APPLYING or DONE.
