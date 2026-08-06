# Applier resolution — S012557, attempt 2

## Result

ACCEPTED without source modification.  The candidate is a complete translation
of the only implementation-bearing declaration in the pinned oracle,
`include/asm-generic/mcs_spinlock.h:4-8`.  It preserves the required MCS node
representation and deliberately does not invent the lock operations that the
oracle does not define.

## Independent source recheck

I reopened the complete pinned header at revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the S012557 scope/symbol/ABI/
lifetime records, both frozen configurations, and its selected direct locking
contexts.  Both configurations select SMP, `ARCH_USE_QUEUED_SPINLOCKS`, and
`QUEUED_SPINLOCKS`; header-closure evidence selects this header for
`kernel/locking/qspinlock.c` on both targets and `kernel/bpf/rqspinlock.c` on
aarch64.

The pinned declaration has exactly these members, in order: a nullable
non-owning `struct mcs_spinlock *next`, then `int locked`, then `int count`.
The candidate has the same order under `#[repr(C)]`, with
`UnsafeCell<*mut mcs_spinlock>`, `UnsafeCell<c_int>`, and
`UnsafeCell<c_int>`.  `UnsafeCell<T>` is representation-transparent and does
not add ownership, drop behavior, a lock algorithm, or a memory ordering.  On
both approved 64-bit targets this retains the pointer/int/int layout required
by the qspinlock context: offset 0 for `next`, offsets 8 and 12 for the two
`int` members, 8-byte alignment, and 16-byte aggregate size.

The source-level synchronization closure is intentionally outside this
declaration but fixed by its direct users.  Generic MCS initialization writes
`locked = 0` and `next = NULL` before the full-barrier `xchg` publication;
the predecessor links `prev->next` with `WRITE_ONCE`; the waiter observes its
`locked` field through the architecture acquire operation; and unlock reads
`next` with `READ_ONCE` before release-storing `1` to the successor's
`locked`.  qspinlock likewise establishes its initialization-before-publication
barrier and uses `count` as the per-CPU nesting counter.  Therefore making
these fields Rust atomic types, assigning an ordering here, or adding a safe
locking API would change the upstream split and is rejected.  `UnsafeCell`
keeps the shared mutable storage from being represented as ordinarily immutable
Rust data; the explicit `Send`/`Sync` contracts limit cross-CPU use to the
matching raw one-copy/acquire/release operations.  The successor link remains
intrusive and non-owning, so neither a Rust reference lifetime nor `Drop` may
control the linked node.

## Review-finding dispositions

| Review | Finding | Disposition |
| --- | --- | --- |
| Parity, slot 1 | None | Confirmed: no source symbol, field, selected branch, operation, or ABI element is omitted or added. |
| Rust, slot 2 | None | Confirmed: `#[repr(C)]`, transparent shared-mutation storage, raw non-owning link, and narrowly documented unsafe auto-trait assertions preserve the source contract. |

## Semantic-record closures

- `struct mcs_spinlock` ownership/lifetime: an externally owned intrusive MCS
  node.  It may be a caller-local generic-MCS node or a per-CPU qspinlock node;
  it must outlive its publication through the corresponding unlock hand-off.
- Concurrency/refcount/RCU: no refcount or RCU ownership is present.  The
  predecessor/successor hand-off is synchronized solely by the named
  one-copy, exchange, acquire, release, and barrier operations in the locking
  consumers.
- ABI: C aggregate order is self-pointer, C `int`, C `int`; the selected
  targets require the 16-byte, alignment-8 representation above.  This header
  declares no linkage, callable symbol, or operative lock macro.
- Selection: there is no configuration-dependent field or code branch beyond
  the C include guard, which Rust's module system does not reproduce as a
  runtime or ABI feature.

No compiler, formatter, linker, analyzer, test, runtime, or benchmark command
was run during this applier pass.
