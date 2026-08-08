// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: kernel/locking/lock_events_list.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016888

/// Expands the frozen configuration union's ordered locking-event list into
/// the callback macro supplied by its consumer.
///
/// The pinned x86_64 and AArch64 configurations both select queued
/// spinlocks and neither selects paravirtual spinlocks.  The omitted PV
/// entries are therefore not part of this translation task's selected union.
#[macro_export]
macro_rules! lock_events_list {
    ($lock_event:ident) => {
        $lock_event! {
            lock_pending,
            lock_slowpath,
            lock_use_node2,
            lock_use_node3,
            lock_use_node4,
            lock_no_node,
            rqspinlock_lock_timeout,
            rwsem_sleep_reader,
            rwsem_sleep_writer,
            rwsem_wake_reader,
            rwsem_wake_writer,
            rwsem_opt_lock,
            rwsem_opt_fail,
            rwsem_opt_nospin,
            rwsem_rlock,
            rwsem_rlock_steal,
            rwsem_rlock_fast,
            rwsem_rlock_fail,
            rwsem_rlock_handoff,
            rwsem_wlock,
            rwsem_wlock_fail,
            rwsem_wlock_handoff,
            rtlock_slowlock,
            rtlock_slow_acq1,
            rtlock_slow_acq2,
            rtlock_slow_sleep,
            rtlock_slow_wake,
            rtmutex_slowlock,
            rtmutex_slow_block,
            rtmutex_slow_acq1,
            rtmutex_slow_acq2,
            rtmutex_slow_acq3,
            rtmutex_slow_sleep,
            rtmutex_slow_wake,
            rtmutex_deadlock,
            lockdep_acquire,
            lockdep_lock,
            lockdep_nocheck,
        }
    };
}
