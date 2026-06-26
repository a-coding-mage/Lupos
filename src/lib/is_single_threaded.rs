//! linux-parity: complete
//! linux-source: vendor/linux/lib/is_single_threaded.c
//! test-origin: linux:vendor/linux/lib/is_single_threaded.c
//! Thread-group single-mm predicate model.

pub const PF_KTHREAD: u32 = 0x0020_0000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ThreadGroupSnapshot {
    pub live_threads: u32,
    pub mm_users: u32,
    pub other_task_shares_mm: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TaskSnapshot {
    pub mm: Option<u32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcessSnapshot<'a> {
    pub flags: u32,
    pub is_current_group_leader: bool,
    pub threads: &'a [TaskSnapshot],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SingleThreadedScanReport {
    pub single_threaded: bool,
    pub rcu_locked: bool,
    pub rcu_unlocked: bool,
    pub smp_rmb_calls: usize,
    pub scanned_processes: usize,
    pub scanned_threads: usize,
}

pub const fn current_is_single_threaded_snapshot(snapshot: ThreadGroupSnapshot) -> bool {
    if snapshot.live_threads != 1 {
        return false;
    }
    if snapshot.mm_users == 1 {
        return true;
    }
    !snapshot.other_task_shares_mm
}

pub fn current_is_single_threaded_detailed(
    live_threads: u32,
    current_mm: u32,
    mm_users: u32,
    processes: &[ProcessSnapshot<'_>],
) -> SingleThreadedScanReport {
    if live_threads != 1 {
        return SingleThreadedScanReport {
            single_threaded: false,
            rcu_locked: false,
            rcu_unlocked: false,
            smp_rmb_calls: 0,
            scanned_processes: 0,
            scanned_threads: 0,
        };
    }

    if mm_users == 1 {
        return SingleThreadedScanReport {
            single_threaded: true,
            rcu_locked: false,
            rcu_unlocked: false,
            smp_rmb_calls: 0,
            scanned_processes: 0,
            scanned_threads: 0,
        };
    }

    let mut report = SingleThreadedScanReport {
        single_threaded: true,
        rcu_locked: true,
        rcu_unlocked: false,
        smp_rmb_calls: 0,
        scanned_processes: 0,
        scanned_threads: 0,
    };

    for process in processes {
        report.scanned_processes += 1;
        if process.flags & PF_KTHREAD != 0 {
            continue;
        }
        if process.is_current_group_leader {
            continue;
        }

        for thread in process.threads {
            report.scanned_threads += 1;
            match thread.mm {
                Some(mm) if mm == current_mm => {
                    report.single_threaded = false;
                    report.rcu_unlocked = true;
                    return report;
                }
                Some(_) => break,
                None => report.smp_rmb_calls += 1,
            }
        }
    }

    report.rcu_unlocked = true;
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_threaded_predicate_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/is_single_threaded.c"
        ));
        assert!(source.contains("atomic_read(&task->signal->live) != 1"));
        assert!(source.contains("atomic_read(&mm->mm_users) == 1"));
        assert!(source.contains("for_each_process(p)"));
        assert!(source.contains("unlikely(p->flags & PF_KTHREAD)"));
        assert!(source.contains("unlikely(p == task->group_leader)"));
        assert!(source.contains("for_each_thread(p, t)"));
        assert!(source.contains("t->mm == mm"));
        assert!(source.contains("if (likely(t->mm))"));
        assert!(source.contains("break;"));
        assert!(source.contains("smp_rmb();"));
        let sched_h = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/sched.h"
        ));
        assert!(sched_h.contains("#define PF_KTHREAD\t\t0x00200000"));
        assert_eq!(PF_KTHREAD, 0x0020_0000);

        assert!(!current_is_single_threaded_snapshot(ThreadGroupSnapshot {
            live_threads: 2,
            mm_users: 1,
            other_task_shares_mm: false,
        }));
        assert!(current_is_single_threaded_snapshot(ThreadGroupSnapshot {
            live_threads: 1,
            mm_users: 1,
            other_task_shares_mm: true,
        }));
        assert!(!current_is_single_threaded_snapshot(ThreadGroupSnapshot {
            live_threads: 1,
            mm_users: 3,
            other_task_shares_mm: true,
        }));
        assert!(current_is_single_threaded_snapshot(ThreadGroupSnapshot {
            live_threads: 1,
            mm_users: 3,
            other_task_shares_mm: false,
        }));
    }

    #[test]
    fn detailed_scan_matches_process_thread_rules() {
        let skipped_kernel = ProcessSnapshot {
            flags: PF_KTHREAD,
            is_current_group_leader: false,
            threads: &[TaskSnapshot { mm: Some(7) }],
        };
        let skipped_leader = ProcessSnapshot {
            flags: 0,
            is_current_group_leader: true,
            threads: &[TaskSnapshot { mm: Some(7) }],
        };
        let null_then_other = ProcessSnapshot {
            flags: 0,
            is_current_group_leader: false,
            threads: &[TaskSnapshot { mm: None }, TaskSnapshot { mm: Some(9) }],
        };
        let sharing_mm = ProcessSnapshot {
            flags: 0,
            is_current_group_leader: false,
            threads: &[TaskSnapshot { mm: Some(7) }],
        };

        assert_eq!(
            current_is_single_threaded_detailed(
                1,
                7,
                3,
                &[skipped_kernel, skipped_leader, null_then_other]
            ),
            SingleThreadedScanReport {
                single_threaded: true,
                rcu_locked: true,
                rcu_unlocked: true,
                smp_rmb_calls: 1,
                scanned_processes: 3,
                scanned_threads: 2,
            }
        );

        assert_eq!(
            current_is_single_threaded_detailed(1, 7, 3, &[sharing_mm]),
            SingleThreadedScanReport {
                single_threaded: false,
                rcu_locked: true,
                rcu_unlocked: true,
                smp_rmb_calls: 0,
                scanned_processes: 1,
                scanned_threads: 1,
            }
        );
    }
}
