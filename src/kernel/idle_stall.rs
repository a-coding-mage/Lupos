//! linux-parity: stub
//! linux-source: vendor/linux/kernel/hung_task.c
//! test-origin: lupos-specific:no Linux counterpart, see rationale below
//! Fully-idle stall detector — "every CPU is halted but userspace is waiting".
//!
//! # Why this exists (Lupos-specific, no direct Linux counterpart)
//!
//! Linux detects a wedged task with `CONFIG_DETECT_HUNG_TASK`
//! (`vendor/linux/kernel/hung_task.c`): `khungtaskd` walks the task list and
//! reports anything that stayed in `TASK_UNINTERRUPTIBLE` past
//! `hung_task_timeout_secs`. Lupos parses the `hung_task_panic` boot sysctl in
//! `src/init/boot.rs` but has no detector behind it, so nothing reports here.
//!
//! A plain port of `hung_task.c` would also not see the failure this file was
//! written for. The boot stall under investigation leaves the machine **fully
//! idle**: sampling all four vCPUs 80 times during a
//! `A start job is running for User Runtime Directory` stall put 80/80 samples
//! in `cpu_startup_entry`, every CPU halted, while the serial log advanced and
//! the systemd job counter ran on. The waiter is not spinning and not in D
//! state — it is asleep on a wakeup that never arrived, which is precisely the
//! case `check_hung_uninterruptible_tasks()` filters out.
//!
//! So the trigger here is the observed signature rather than Linux's: every
//! active CPU parked in the idle loop, no task in `TASK_RUNNING`, for longer
//! than the threshold. That condition is already computable — `all_cpus_idle()`
//! in `kernel/sched/nohz.rs` has carried no production consumer since M31.
//!
//! # Why not just trace syscalls
//!
//! `lupos.trace=syscall` does record which syscall each task blocked in, and it
//! does show PID 1 parked in `epoll_wait(..., timeout=-1)`. But the tracing
//! itself destroys the measurement: ~46 000 serial writes over a boot keep the
//! CPUs awake and keep interrupts flowing, and the largest idle gap observed in
//! a fully traced boot was 312 jiffies (1.25 s at HZ=250) — no multi-second
//! stall reproduced at all. A detector that stays silent until the stall fires
//! is the only way to observe a lost wakeup without supplying the very wakeups
//! that hide it.
//!
//! Opt-in via `lupos.trace=stall`, so a normal boot pays one relaxed atomic
//! load per idle-loop pass and prints nothing. This matters because an idle
//! system is legitimately fully idle for long stretches — the graphics gate
//! sits at a password prompt — and those are not stalls.
//!
//! The dump format follows Linux `sched_show_task()` / `show_state_filter()`.

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::kernel::task::{TaskStruct, task_state};

/// Jiffies of full idleness before the stall is reported. HZ is 250, so this
/// is 2 s — well under the 24 s / 59 s stalls being chased, and far above any
/// ordinary idle gap between a wakeup and its handler.
const STALL_THRESHOLD_JIFFIES: u64 = 500;

/// Jiffies stamp when the machine last became fully idle; 0 means "not idle".
static IDLE_SINCE: AtomicU64 = AtomicU64::new(0);

/// One report per stall episode, cleared when the machine runs again.
static REPORTED: AtomicBool = AtomicBool::new(false);

/// Linux `task_state_to_char()` letters, from `sched_show_task()`.
fn state_char(state: u32) -> u8 {
    if state == task_state::TASK_RUNNING {
        return b'R';
    }
    if state & task_state::__TASK_TRACED != 0 {
        return b't';
    }
    if state & task_state::__TASK_STOPPED != 0 {
        return b'T';
    }
    if state & task_state::TASK_DEAD != 0 {
        return b'X';
    }
    if state & task_state::TASK_UNINTERRUPTIBLE != 0 {
        return b'D';
    }
    if state & task_state::TASK_INTERRUPTIBLE != 0 {
        return b'S';
    }
    b'?'
}

fn comm_str(task: *mut TaskStruct) -> &'static str {
    let comm = unsafe { &(*task).comm };
    let len = comm.iter().position(|&c| c == 0).unwrap_or(comm.len());
    core::str::from_utf8(&comm[..len]).unwrap_or("?")
}

/// Walk every task registry and report each one's state.
///
/// Both registries are needed: `for_each_pool_task()` only covers kthreads in
/// `TASK_POOL`/`AP_IDLE_TASKS`, while userspace tasks are heap-allocated and
/// tracked separately in `kernel::fork::HEAP_TASKS`.
fn for_each_task(mut f: impl FnMut(*mut TaskStruct)) {
    crate::kernel::sched::for_each_pool_task(&mut f);
    crate::kernel::fork::for_each_heap_task(&mut f);
}

/// True when no task anywhere is `TASK_RUNNING`.
///
/// This is what separates a real stall from "all CPUs happened to be between
/// tasks". A lost wakeup leaves every task blocked with nothing to schedule.
///
/// The per-CPU idle tasks (`swapper/0`, `swapper/ap`, all `pid == 0`) are
/// excluded: `for_each_pool_task()` enumerates `AP_IDLE_TASKS`, and an idle
/// task is by definition `TASK_RUNNING` on its halted CPU, so counting them
/// would make this predicate permanently false.
fn no_runnable_task() -> bool {
    let mut runnable = false;
    for_each_task(|task| {
        if unsafe { (*task).pid } <= 0 {
            return;
        }
        if unsafe { (*task).__state.load(Ordering::Relaxed) } == task_state::TASK_RUNNING {
            runnable = true;
        }
    });
    !runnable
}

/// Note on locking: this runs from the idle task and takes `HEAP_TASKS` and
/// (via `has_pending_signal_for_pid`) `SIGNAL_TABLE`. That is safe only because
/// it runs when nothing is runnable — a task blocked while holding either lock
/// would wedge the idle loop here instead of reporting. If this file ever
/// hard-hangs the machine at the stall, that is itself the finding.
#[cfg(not(test))]
fn report(idle_jiffies: u64) {
    use crate::linux_driver_abi::tty::serial_println;

    // A non-zero runnable count here is the whole point: those tasks are
    // marked runnable while this CPU has had nothing to do for seconds, i.e.
    // they were made runnable but never enqueued anywhere a CPU would find
    // them. Zero runnable is an ordinary idle machine.
    let mut runnable = 0u32;
    let mut lost = 0u32;
    for_each_task(|task| {
        if unsafe { (*task).pid } <= 0
            || unsafe { (*task).__state.load(Ordering::Relaxed) } != task_state::TASK_RUNNING
        {
            return;
        }
        runnable += 1;
        // Runnable but on no runqueue: the state that cannot recover, because
        // every later wakeup short-circuits on "already TASK_RUNNING".
        if unsafe { (*task).m29.on_rq } == 0 {
            lost += 1;
        }
    });
    serial_println!(
        "INFO: cpu idle for {}.{:02}s, runnable={} runnable_off_rq={}",
        idle_jiffies / crate::kernel::time::jiffies::HZ,
        (idle_jiffies % crate::kernel::time::jiffies::HZ) * 100 / crate::kernel::time::jiffies::HZ,
        runnable,
        lost
    );
    for_each_task(|task| {
        let state = unsafe { (*task).__state.load(Ordering::Relaxed) };
        let pid = unsafe { (*task).pid };
        // SIGCHLD pending on a *sleeping* task is the decisive datum: systemd
        // waits for its forked helpers through SIGCHLD on a signalfd inside
        // epoll, so "pending but asleep" means the readiness never reached the
        // epoll waiter and only the sd-event timerfd will free it.
        let sigchld = pid > 0
            && crate::kernel::signal::has_pending_signal_for_pid(
                pid,
                crate::kernel::signal::SIGCHLD,
            );
        // `on_rq` is the proof. Linux's invariant is that a TASK_RUNNING task
        // is on a runqueue, and `wake_task_normal()` relies on it: it returns
        // early for a task whose state is already TASK_RUNNING. A task showing
        // state:R with on_rq:0 while every CPU sits idle has been made
        // runnable and then lost — nothing will ever pick it up, and any
        // further wakeup is a no-op.
        serial_println!(
            "  task:{:<16} state:{} ({:#06x}) on_rq:{} pid:{} tgid:{} sigchld_pending:{}",
            comm_str(task),
            state_char(state) as char,
            state,
            unsafe { (*task).m29.on_rq },
            pid,
            unsafe { (*task).tgid },
            sigchld as u8
        );
        // For an uninterruptible sleeper, name *where* it is blocked. Lupos has
        // no `/proc/<pid>/wchan` and no `kallsyms`, but neither is needed: dump
        // the raw return addresses from the saved kernel stack and symbolize
        // them on the host with
        //   addr2line -f -C -e target/xtask/cargo-graphics-x11/x86_64-lupos/release/lupos <addr>
        //
        // A `D` task is descheduled, so `thread.sp` is stable and its stack is
        // quiescent. Only kernel-text-looking words are printed, and the count
        // is capped, so this stays a few lines per stalled task.
        if state & task_state::TASK_UNINTERRUPTIBLE != 0 {
            let sp = unsafe { (*task).thread.sp };
            if sp != 0 {
                let mut shown = 0u32;
                for slot in 0..64u64 {
                    if shown >= 8 {
                        break;
                    }
                    let addr = sp + slot * 8;
                    // SAFETY: reading the descheduled task's own kernel stack.
                    let word = unsafe { core::ptr::read_volatile(addr as *const u64) };
                    if (0x20_0000..0x100_0000).contains(&word) {
                        serial_println!("      blocked-at pid:{} retaddr:{:#x}", pid, word);
                        shown += 1;
                    }
                }
            }
        }
    });
}

/// Minimum gap between two reports, so a legitimately idle machine (a greeter
/// waiting for a password) produces an occasional line instead of a flood.
const REPORT_INTERVAL_JIFFIES: u64 = 2500;

/// Jiffies stamp of the last report, for rate limiting.
static LAST_REPORT: AtomicU64 = AtomicU64::new(0);

/// Positive control. Without it, "no stall reported" is ambiguous between
/// "the machine never went fully idle" and "the detector is broken" — and the
/// first version of this file was in fact broken (it counted the `pid == 0`
/// idle tasks as runnable, so the predicate could never hold). Printed once.
static ARMED: AtomicBool = AtomicBool::new(false);

/// Called from every pass of `do_idle()`, with the jiffies stamp taken when
/// this CPU *entered* `do_idle()`. Cheap and silent until the threshold.
///
/// `idle_since` comes from the caller because `do_idle()` only returns when
/// `need_resched` is set, so "this CPU has been in the idle loop continuously
/// since T" is a purely local fact with no cross-CPU race. Two earlier keys
/// were tried and both failed:
///
/// * `all_cpus_idle()` — the nohz mask is cleared for a window on every softirq
///   pass, and four CPUs sampling it ~1000×/s meant one transient observation
///   reset the timer, so the threshold was unreachable.
/// * "no task is `TASK_RUNNING`" — this *did* arm, but never fired across a
///   real 108 s stall, because during the stall a task **is** in `TASK_RUNNING`
///   the whole time while every CPU sits halted. That is the finding, not a
///   detector failure: a runnable task that no CPU ever picks up. Gating on the
///   absence of such a task hid exactly the case worth seeing.
#[cfg(not(test))]
pub fn idle_stall_check(cpu: u32, idle_since: u64) {
    if !crate::kernel::debug_trace::stall_enabled() {
        return;
    }
    // Any CPU may report, not just the do_timer CPU: during the stall PID 1
    // stays alive redrawing systemd's progress spinner, so CPU 0 is not
    // reliably the one sitting idle. The global rate limit below keeps output
    // bounded no matter how many CPUs qualify.
    let _ = cpu;

    let now = crate::kernel::time::jiffies::jiffies();
    if now.saturating_sub(idle_since) < STALL_THRESHOLD_JIFFIES {
        return;
    }
    // compare_exchange, not load/store: several CPUs can satisfy the idle
    // threshold at once, and only one of them may print.
    let last = LAST_REPORT.load(Ordering::Relaxed);
    if last != 0 && now.saturating_sub(last) < REPORT_INTERVAL_JIFFIES {
        return;
    }
    if LAST_REPORT
        .compare_exchange(last, now, Ordering::AcqRel, Ordering::Relaxed)
        .is_err()
    {
        return;
    }
    IDLE_SINCE.store(idle_since, Ordering::Relaxed);
    REPORTED.store(true, Ordering::Relaxed);
    ARMED.store(true, Ordering::Relaxed);
    report(now.saturating_sub(idle_since));
}

#[cfg(test)]
pub fn idle_stall_check(_cpu: u32, _idle_since: u64) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_char_matches_linux_letters() {
        assert_eq!(state_char(task_state::TASK_RUNNING), b'R');
        assert_eq!(state_char(task_state::TASK_INTERRUPTIBLE), b'S');
        assert_eq!(state_char(task_state::TASK_UNINTERRUPTIBLE), b'D');
        assert_eq!(state_char(task_state::TASK_DEAD), b'X');
    }
}
