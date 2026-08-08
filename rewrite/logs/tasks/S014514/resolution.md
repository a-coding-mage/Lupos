# S014514 applier resolution — attempt 2 / P01

## Scope and evidence

This adjudication reopened the complete pinned
`vendor/linux/include/linux/nfs_iostat.h` at Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the current candidate, the
attempt-2 implementation and candidate records, and both current review
reports.  It also inspected the direct pinned consumers
`vendor/linux/fs/nfs/iostat.h` and `vendor/linux/fs/nfs/super.c`, plus the
`seq_printf` declaration in `vendor/linux/include/linux/seq_file.h`.

The candidate snapshot attested by the current reviews is
`c0feee9a70766fbe51b48fc9037490c2fd59fc0769a32c28f0088ff40ae77ff4`.
The current source file was not changed during application.  The semantic
proposal is sealed as
`d250907c4b6a96c3c977b6699b8d2fd8811c9a4867693061a0fe121bc6fe5484`;
the parity and Rust review report hashes are respectively
`49e28910293cbb08e3469ebe376e5ea59b3f6a88ee0d1b9ce07b95a90edfeb6e` and
`7eaad0f612d8bf7943fe792c1e26e23de6916ef902716d286d4734eb2fa8ab02`.
No compiler, formatter, linker, test, runtime, rust-analyzer diagnostic, or
historical Rust source was used.

## Finding dispositions

### F1 — `NFS_IOSTAT_VERS` representation

**Disposition: SUSTAINED; unresolved exact translation, blocks the task.**

Pinned source line 25 defines `NFS_IOSTAT_VERS` as the C string literal
`"1.1"`.  Its direct selected consumer is
`fs/nfs/super.c:662`, which passes it as the `%s` variadic argument to
`seq_printf`; `include/linux/seq_file.h:111-114` declares that argument as a
C format string plus variadic arguments.  Therefore the source establishes a
NUL-terminated C-string expression at this use.  The candidate's `&str` does
not encode the terminator and is not the C pointer expression used by that
call.

An explicit terminated byte array, a pointer constant, a `CStr`-based API, or
another Rust-side adaptation would each change the public item and the caller
contract.  The selected translated caller and the frozen records provide no
source-proven contract that chooses one of those designs.  Applying one here
would be a new unreviewed design, would invalidate the current candidate and
both reviews, and would still leave the precise Rust FFI/use boundary
unestablished.  It is consequently not safe to correct this finding in this
attempt.

### RR-001 — `NFS_IOSTAT_VERS` C-string compatibility

**Disposition: SUSTAINED; same blocker as F1.**

The independent Rust review identifies the same mismatch.  The direct caller
evidence above confirms the required C-string use rather than disproving it.
No separate resolution is possible: the source-only record does not establish
the exact translated representation and use contract needed to replace the
candidate's `&str` without a design change.

### F2 — `_LINUX_NFS_IOSTAT` include guard

**Disposition: SUSTAINED; unresolved exact translation, blocks the task.**

Pinned source lines 22-23 and 122 implement a C preprocessor conditional and
definition: a first textual inclusion defines `_LINUX_NFS_IOSTAT`, and later
inclusions suppress all declarations.  The frozen symbol rows select the
conditional and macro for both architectures.  The candidate has no mapping
for either operation.

The direct source consumer `fs/nfs/iostat.h:16` shows C textual inclusion, but
the authorized source set contains no established Rust module-loading or
cross-language include contract proving that Rust module loading is equivalent
to this observable preprocessor state.  Inventing a Rust macro or module guard
would be a new design, not an upstream-derived correction.  Exact parity is
therefore not established source-only.

### F3 — named C enum types and ABI

**Disposition: SUSTAINED; unresolved exact translation, blocks the task.**

Pinned source defines distinct named types `enum nfs_stat_bytecounters` at
line 62 and `enum nfs_stat_eventcounters` at line 91.  The direct consumer
`fs/nfs/iostat.h:23-46` uses each distinct enum tag as a function-parameter
type and uses their values to index separate arrays.  The candidate aliases
both tags to `i32`, erasing the nominal distinction.

The frozen ABI proposal's records for both enum types and both architectures
(ABI base rows 137048-137051) set layout and alignment only to the literal
`SOURCE_REVIEWED_VALUE`; they do not supply an architecture-specific value or
an upstream declaration establishing one.  The pinned header itself likewise
does not specify a fixed representation or alignment.  Selecting `i32`,
`#[repr(C)]`, a transparent newtype, or another representation without the
missing exact ABI evidence would guess.  The pending ABI and related lifetime
closures therefore cannot be finalized for this task.

## Terminal recommendation

Do **not** mark S014514 `DONE` and do not commit the semantic-closure proposal.
The candidate and all review artifacts remain intact; no source or candidate
artifact was changed.  The queue owner should issue exactly this terminal
transition through the queue tool:

```text
python3 tools/rewrite_queue.py block --id S014514 --pipeline P01 --reason "F1/RR-001 C-string macro representation and F2 include-guard mapping lack a source-proven Rust contract; F3 enum ABI/layout remains unestablished for x86_64 and aarch64"
```

This is a Phase 1 source-evidence blocker, not a build or test result.
