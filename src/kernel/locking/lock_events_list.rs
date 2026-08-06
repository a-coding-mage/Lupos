// SPDX-License-Identifier: GPL-2.0
//! linux-source: kernel/locking/lock_events_list.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016888

// Upstream author: Waiman Long <longman@redhat.com>

/// Applies a consumer macro to the selected ordered `LOCK_EVENT` list.
///
/// This is the Rust continuation form of the source X-macro fragment: its
/// caller owns the expansion product.  In particular, `lock_events.h` owns
/// the enum and its trailing `lockevent_num` / `LOCKEVENT_reset_cnts` entries;
/// a C source which needs a names table owns that table and its reset entry.
/// Both frozen configurations select queued spinlocks and do not select PV
/// spinlocks, so this list has the selected non-PV entries in source order.
#[macro_export]
macro_rules! lock_event_list {
    ($consumer:ident) => {
        $consumer!(
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
        )
    };
}
