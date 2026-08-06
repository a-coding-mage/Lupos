# Rust review — S012557 attempt 2

## Outcome

ACCEPTED. No Rust-specific source finding in
`src/include/asm-generic/mcs_spinlock.rs`.

## Evidence reviewed

- Pinned declaration: `vendor/linux/include/asm-generic/mcs_spinlock.h:4-8`.
  It contains, in order, `struct mcs_spinlock *next`, `int locked`, and
  `int count`.
- Candidate: `src/include/asm-generic/mcs_spinlock.rs:17-34`.
- Selected consumers and synchronization contract:
  `vendor/linux/kernel/locking/mcs_spinlock.h:57-112` and
  `vendor/linux/kernel/locking/qspinlock.c:196-274`.  The former performs the
  successor publication through `WRITE_ONCE`, waits on `locked` with acquire
  semantics, and hands off with release semantics.  The latter uses the node
  as an intrusive, per-CPU queue entry and publishes it only after its
  initialization barrier.
- Both frozen configurations select SMP and queued spinlocks.  The generic
  header is selected for both architectures; it has no configuration-dependent
  member or branch.

## Rust audit

`#[repr(C)]` keeps declaration ordering and C aggregate layout.  On each
approved 64-bit target, the raw successor pointer followed by two `c_int`
fields therefore retains the C node's pointer/int/int representation (the
consumer documents its 16-byte 64-bit-node expectation).  `UnsafeCell<T>` is
layout-transparent for these field representations while preventing a shared
Rust reference from asserting immutable access to fields Linux changes through
its synchronization protocol.

The raw `next` link correctly retains the source's non-owning intrusive-node
semantics: it introduces neither a Rust lifetime nor drop ownership over a
successor.  The node has no `Drop`, `Copy`, or safe field-load/store API that
could move, free, or race a linked entry.  The explicit `Send`/`Sync` impls are
necessary because the Linux protocol shares a node between the owning CPU and
its predecessor/successor; their `SAFETY` comments correctly restrict useful
field access to the matching raw synchronization operations.  Safe callers can
obtain only raw field addresses from `UnsafeCell`; dereferencing them remains
unsafe.

This declaration deliberately does not select atomic orderings itself.  That
matches the upstream split: ordering belongs to the `READ_ONCE`/`WRITE_ONCE`,
acquire, release, barrier, and exchange translations in the locking consumer
tasks.  Those later translations must not turn the `UnsafeCell` addresses into
ordinary concurrent Rust loads or stores: doing so would be a Rust data race.
They must preserve the named one-copy/acquire/release operations and the
publication-before-link ordering from the pinned sources.  This is a
downstream implementation obligation, not a defect in this declaration-only
task.

No compiler, formatter, linker, test, or runtime command was run.
