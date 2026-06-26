//! linux-parity: partial
//! linux-source: vendor/linux/kernel/locking
//! test-origin: linux:vendor/linux/kernel/locking
//! Locking primitives — M33.
//!
//! Mirrors `vendor/linux/kernel/locking/`.
//!
//! | Module       | Linux source                | Description                          |
//! |--------------|-----------------------------|--------------------------------------|
//! | `preempt`    | include/linux/preempt.h     | `preempt_count`, in_atomic           |
//! | `irqflags`   | arch/x86/include/asm/irqflags.h | local_irq_save/restore           |
//! | `raw_spinlock` | (ticket impl)             | `raw_spinlock_t` (IRQ-safe)          |
//! | `spinlock`   | qspinlock.c                 | `spinlock_t` (preempt + bh)          |
//! | `mutex`      | mutex.c                     | sleeping mutex                       |
//! | `rwsem`      | rwsem.c                     | reader/writer semaphore              |
//! | `semaphore`  | semaphore.c                 | counting semaphore                   |
//! | `completion` | completion.c                | one-shot completion                  |
//! | `rt_mutex`   | rtmutex.c                   | priority-inheritance mutex (futex PI)|
//! | `wake_q`     | sched/wake_q.c              | batched wake list                    |

pub mod completion;
pub mod irqflag_debug;
pub mod irqflags;
pub mod lock_events;
pub mod lockdep;
pub mod lockdep_proc;
pub mod locktorture;
pub mod mutex;
pub mod mutex_debug;
pub mod osq_lock;
pub mod percpu_rwsem;
pub mod preempt;
pub mod qrwlock;
pub mod qspinlock;
pub mod raw_spinlock;
pub mod rt_mutex;
pub mod rtmutex_api;
pub mod rwbase_rt;
pub mod rwsem;
pub mod semaphore;
pub mod spinlock;
pub mod spinlock_debug;
pub mod spinlock_rt;
pub mod test_ww_mutex;
pub mod wake_q;
pub mod ww_rt_mutex;

pub use completion::Completion;
pub use irqflags::{
    IrqFlags, X86_EFLAGS_IF, irqs_disabled, irqs_disabled_flags, local_irq_disable,
    local_irq_enable, local_irq_restore, local_irq_save,
};
pub use mutex::{Mutex, MutexGuard};
pub use preempt::{
    HARDIRQ_OFFSET, NMI_OFFSET, PREEMPT_OFFSET, SOFTIRQ_OFFSET, in_atomic, in_hardirq, in_irq,
    in_nmi, in_softirq, local_bh_disable, local_bh_enable, might_sleep, preempt_count,
    preempt_disable, preempt_enable,
};
pub use raw_spinlock::{RawSpinGuard, RawSpinLock, RawSpinLocked};
pub use rt_mutex::{PiState, RT_MUTEX_HAS_WAITERS, RtMutex, RtMutexWaiter};
pub use rwsem::{RwReadGuard, RwSem, RwWriteGuard};
pub use semaphore::Semaphore;
pub use spinlock::{SpinGuard, SpinLock};
pub use wake_q::WakeQHead;

#[cfg(test)]
mod tests {
    #[test]
    fn sleepable_locks_schedule_with_irqs_enabled() {
        for (name, source) in [
            ("completion", include_str!("completion.rs")),
            ("mutex", include_str!("mutex.rs")),
            ("rt_mutex", include_str!("rt_mutex.rs")),
            ("rwsem", include_str!("rwsem.rs")),
            ("semaphore", include_str!("semaphore.rs")),
        ] {
            assert!(
                source.contains("schedule_with_irqs_enabled"),
                "{name} must use the IRQ-open scheduler helper for blocking waits"
            );
            assert!(
                !source.contains("crate::kernel::sched::schedule();"),
                "{name} must not call bare schedule from sleepable lock waits"
            );
        }
    }
}
