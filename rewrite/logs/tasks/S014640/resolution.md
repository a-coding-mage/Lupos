# Applier resolution — S014640

Task `S014640` (`include/linux/pid_types.h` ->
`src/include/linux/pid_types.rs`) is **BLOCKED**.  This is a source-only
adjudication against pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`; no compiler, formatter, analyzer,
linker, test, debugger, or runtime command was used.

The required branch is `feat/bun-like-rewrite-test`.  The frozen task row is
common to x86_64 and aarch64.  Both frozen configurations select
`CONFIG_PID_NS=y`, `CONFIG_SYSCTL=y`, and `CONFIG_MEMFD_CREATE=y`.

## Disposition of parity finding 1

**Accepted; candidate rejected.**  Pinned
`include/linux/pid_types.h:13-14` contains only the incomplete declaration
`struct pid_namespace;` followed by `extern struct pid_namespace init_pid_ns;`.
The candidate's `#[repr(C)] struct pid_namespace { _private: [u8; 0] }` makes
that incomplete type a concrete zero-sized Rust definition, which is not the
Linux object type.

Pinned `include/linux/pid_namespace.h:26-50` owns the complete
`struct pid_namespace` definition.  It is nonzero and, under both selected
configuration sets, includes the `CONFIG_SYSCTL` plus `CONFIG_MEMFD_CREATE`
`memfd_noexec_scope` member.  Pinned `kernel/pid.c:72-84` defines and exports
the mutable object `init_pid_ns`, and later directly accesses its fields.
That defining header maps to separate task `S014639`, which is still `TODO`.

This header cannot define a substitute layout.  Rust's ordinary struct
declaration would be another concrete layout (including a unit/ZST layout),
and the frozen records provide no accepted cross-task Rust declaration/linkage
contract that permits this header to name the eventual canonical complete type
without inventing one.  Therefore the candidate's external-object declaration
cannot be made acceptable within this task at this time.

## Disposition of parity findings 2 and 3

**Accepted; candidate rejected.**  The five enumerators in pinned
`include/linux/pid_types.h:5-11` are ordered values zero through four, but C
also permits the implementation's compatible integer type and its full scalar
object domain.  Pinned users pass `enum pid_type` by value and index with it
(for example `include/linux/pid.h:95-113` and `kernel/pid.c:380-419`).  A
closed Rust enum would impose invalid-discriminant invariants that the C type
does not impose, and it lacks the C scalar's unconditional trivial-copy
semantics.

`rewrite/ABI.tsv:143638-143641` leaves the representation/linkage fields for
both architectures as `PENDING_REVIEW`; the corresponding lifetime rows
`rewrite/LIFETIMES.tsv:139579-139582` are also `PENDING_REVIEW`.  Neither the
frozen ABI record nor a completed direct ABI dependency establishes the exact
Rust scalar representation.  Selecting an `i32`/`c_int` newtype (or retaining
the closed enum) would be an unproven ABI choice.  It must not be invented by
this applier.

## Disposition of Rust finding R1

**Accepted; corrective rework required.**  Pinned
`include/linux/pid_types.h:1` says `SPDX-License-Identifier: GPL-2.0`; the
candidate says `GPL-2.0-only`.  Any requeued candidate must retain the exact
upstream identifier.  This change was not applied because the entire
candidate is rejected for the unresolved ABI blockers above.

## Disposition of Rust finding R2

**Accepted; corrective rework required.**  The task is selected by both
frozen configurations (the per-architecture entries are in
`rewrite/SYMBOLS.tsv:240497-240506`), so a requeued candidate's immutable
provenance must say `architectures: x86_64,aarch64`, not `common`.  This change
was likewise not applied to a candidate that cannot be accepted.

## Final semantic status

No source candidate is accepted.  The unresolved records are now precisely
identified for the blocked task:

- `enum pid_type`: establish the frozen x86_64/aarch64 C compatible integer
  representation and a Rust form that preserves its scalar domain and copy
  behavior.
- `init_pid_ns`: establish the cross-task Rust ownership/declaration contract
  that references the single complete `pid_namespace` representation owned by
  the translation of `include/linux/pid_namespace.h` (S014639), without a
  zero-sized stand-in.

These are source/ABI blockers, not implementation choices.  The queue owner
must transition S014640 from `APPLYING` to `BLOCKED` with this reason; no
`DONE` transition is authorized.
