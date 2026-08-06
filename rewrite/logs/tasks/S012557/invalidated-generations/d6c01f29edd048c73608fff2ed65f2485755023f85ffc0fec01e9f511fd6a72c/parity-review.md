# Parity review — S012557, attempt 2, slot 1

## Verdict

ACCEPT — no parity findings.

## Review boundary

Reviewed only the pinned Linux oracle, frozen Phase 0 task records and
configuration evidence, the current destination source, and direct lock-user
context.  I did not read implementation rationale, candidate diffs, prior
attempt material, or either review report.  This was a manual source review;
no compiler, formatter, linker, test, emulator, debugger, or analyzer was
invoked.

## Oracle and selected scope

- Pinned revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`, matching both
  `vendor/linux.SHA` and the candidate provenance.
- Oracle: `vendor/linux/include/asm-generic/mcs_spinlock.h`, lines 1–19.
- `SYMBOLS.tsv` records exactly the include guard and `struct mcs_spinlock` for
  both x86_64 and aarch64.  The source contains no functions, data objects, or
  operative lock/unlock macro definition to translate.
- Both frozen configurations select `CONFIG_ARCH_USE_QUEUED_SPINLOCKS=y` and
  `CONFIG_QUEUED_SPINLOCKS=y`; x86_64 additionally records
  `CONFIG_PARAVIRT_SPINLOCKS` unset.  Header-closure evidence identifies the
  selected direct consumers as `kernel/locking/qspinlock.c` on both targets and
  `kernel/bpf/rqspinlock.c` on aarch64.

## Structure and ABI comparison

The Oracle has precisely three fields, in order:

| Oracle field | Candidate field | Result |
| --- | --- | --- |
| `struct mcs_spinlock *next` | `UnsafeCell<*mut mcs_spinlock>` | Preserves a nullable self-pointer representation and permits the required shared raw one-copy access. |
| `int locked` | `UnsafeCell<c_int>` | Preserves the C `int` storage representation required for the `0`/`1` hand-off state. |
| `int count` | `UnsafeCell<c_int>` | Preserves the C `int` nesting-counter storage used by qspinlock. |

`#[repr(C)]` fixes declaration order and C field layout.  `UnsafeCell<T>` is a
transparent storage wrapper with T's layout, so on both selected 64-bit Linux
targets the manually derived layout remains `next` at offset 0, `locked` at 8,
`count` at 12, alignment 8, and total size 16.  This agrees with the direct
qspinlock source comment that an `mcs_spinlock` is 16 bytes on 64-bit
architectures.  No padding, field, visibility-relevant type name, or exported
symbol from the source is omitted or added.

## Concurrency, ordering, and direct-user context

The generic header is intentionally a node declaration, not an atomic
implementation.  It only permits architecture definitions of
`arch_mcs_spin_lock_contended` and `arch_mcs_spin_unlock_contended`; it defines
neither macro nor any lock/unlock function.  The candidate correspondingly
adds no replacement algorithm, atomic operation, ordering, timeout, or
side-effect.

The direct generic MCS API in `kernel/locking/mcs_spinlock.h` initializes
`locked`/`next`, publishes a node with `xchg` and `WRITE_ONCE`, waits with the
architecture acquire operation, reads `next` with `READ_ONCE`, and hands off
with the architecture release operation.  `qspinlock.c` uses the same
`next`/`locked` protocol and accesses `count` through per-CPU operations.
`UnsafeCell` correctly keeps these concurrently accessed locations out of
ordinary shared-reference immutability; it neither supplies nor weakens the
required caller-side one-copy/acquire/release primitives.  The explicit
`Send`/`Sync` safety statements are constrained to that Linux hand-off
protocol and do not create a safe field mutation API.  They are necessary for
the shared/per-CPU representation without changing its object representation.

The candidate’s comments faithfully identify these contracts and do not claim
that the declaration itself implements synchronization.  Its SPDX/provenance
header has the required source path, pinned revision, architecture union, and
stable task identifier.  No unauthorized branding, test configuration, stub,
or hidden behavioral replacement is present.

## Finding ledger

No findings.  The future translations that implement `READ_ONCE`/`WRITE_ONCE`,
the MCS acquire/release hand-off, and qspinlock’s per-CPU counter must preserve
the source operations described above; those mechanisms are outside this
declaration-only task and are not missing symbols from its oracle.
