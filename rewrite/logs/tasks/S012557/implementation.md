# S012557 implementation — attempt 2

Source: `vendor/linux/include/asm-generic/mcs_spinlock.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The complete selected header declaration is translated as `#[repr(C)] struct
mcs_spinlock`: `next` first, then `locked`, then `count`.  `c_int` preserves
the C `int` fields. `UnsafeCell<T>` is layout-transparent, preserving the
pointer/int/int ABI while preventing safe Rust shared references from claiming
immutability for fields that Linux accesses through one-copy and
acquire/release primitives.

Selected qspinlock paths establish the required concurrency contract:

- `kernel/locking/qspinlock.c` initializes a per-CPU node, publishes
  `prev->next` with `WRITE_ONCE`, waits on `node->locked` with the
  architecture acquire operation, and later reads `node->next` with
  `READ_ONCE`.
- `kernel/bpf/rqspinlock.c` performs the same MCS publication/hand-off and can
  run in nested task, softirq, hardirq, and NMI contexts.
- `kernel/locking/mcs_spinlock.h` documents the matching exchange, one-copy,
  acquire, and release ordering used by the generic MCS operations.

The binding supplies neither a lock algorithm nor field access helpers: those
belong to the separate kernel locking header/task. `Send` and `Sync` are
explicit because Linux permits this exact cross-CPU hand-off. Their safety
contract requires all access to use the translated Linux raw synchronization
operations; they do not make non-atomic access safe.

No compiler, formatter, linker, test, or runtime command was run.
