//! linux-parity: complete
//! linux-source: vendor/linux/kernel
//! test-origin: linux:vendor/linux/kernel
//! Session and process-group syscall support for early interactive userland.
//!
//! This mirrors the ABI shape of Linux `setsid(2)` and `setpgid(2)` from
//! `kernel/sys.c` / `kernel/pid.c`, while keeping the state in a small side
//! table keyed by PID until `struct pid` grows full PIDTYPE_PGID/PIDTYPE_SID
//! indexes.

extern crate alloc;

use alloc::vec::Vec;
use spin::Mutex;

use crate::include::uapi::errno::{EINVAL, EPERM, ESRCH};
use crate::kernel::{fork, sched};

#[derive(Clone, Copy)]
struct SessionEntry {
    pid: i32,
    pgid: i32,
    sid: i32,
}

static SESSIONS: Mutex<Vec<SessionEntry>> = Mutex::new(Vec::new());

/// The terminal attached to a session.  Linux stores this as
/// `signal_struct::tty`; keeping the stable device identity here gives every
/// process in the session the same `/dev/tty` view without coupling task state
/// to a particular tty implementation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControllingTty {
    Console(u64),
    Unix98Pty(u32, usize),
}

#[derive(Clone, Copy)]
struct ControllingTtyEntry {
    sid: i32,
    tty: ControllingTty,
}

static CONTROLLING_TTYS: Mutex<Vec<ControllingTtyEntry>> = Mutex::new(Vec::new());

fn current_pid() -> Result<i32, i32> {
    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return Err(ESRCH);
    }
    Ok(unsafe { (*task).pid })
}

fn task_exists(pid: i32) -> bool {
    if let Ok(cur) = current_pid() {
        if pid == cur {
            return true;
        }
    }
    !fork::find_heap_task_by_pid(pid).is_null()
}

fn ensure_entry(pid: i32) -> SessionEntry {
    let mut table = SESSIONS.lock();
    if let Some(entry) = table.iter().find(|entry| entry.pid == pid).copied() {
        return entry;
    }
    let entry = SessionEntry {
        pid,
        pgid: pid,
        sid: pid,
    };
    table.push(entry);
    entry
}

fn update_entry(pid: i32, f: impl FnOnce(&mut SessionEntry)) -> Result<SessionEntry, i32> {
    let mut table = SESSIONS.lock();
    if let Some(entry) = table.iter_mut().find(|entry| entry.pid == pid) {
        f(entry);
        return Ok(*entry);
    }
    let mut entry = SessionEntry {
        pid,
        pgid: pid,
        sid: pid,
    };
    f(&mut entry);
    table.push(entry);
    Ok(entry)
}

pub fn process_group(pid: i32) -> Option<i32> {
    let table = SESSIONS.lock();
    table
        .iter()
        .find(|entry| entry.pid == pid)
        .map(|entry| entry.pgid)
}

pub fn session_id(pid: i32) -> Option<i32> {
    let table = SESSIONS.lock();
    table
        .iter()
        .find(|entry| entry.pid == pid)
        .map(|entry| entry.sid)
}

/// Return the controlling terminal visible to `pid` through `/dev/tty`.
pub fn controlling_tty(pid: i32) -> Option<ControllingTty> {
    let sid = session_id(pid).unwrap_or(pid);
    CONTROLLING_TTYS
        .lock()
        .iter()
        .find(|entry| entry.sid == sid)
        .map(|entry| entry.tty)
}

/// Attach `tty` to a session leader that does not already have a controlling
/// terminal.  This is the state transition performed by Linux
/// `tty_open_proc_set_tty()` for a readable tty opened without `O_NOCTTY`.
pub fn claim_controlling_tty(pid: i32, tty: ControllingTty) -> Result<(), i32> {
    let sid = session_id(pid).unwrap_or(pid);
    if pid != sid {
        return Err(EPERM);
    }

    let mut table = CONTROLLING_TTYS.lock();
    if let Some(entry) = table.iter().find(|entry| entry.sid == sid) {
        return if entry.tty == tty { Ok(()) } else { Err(EPERM) };
    }
    if table.iter().any(|entry| entry.tty == tty) {
        return Err(EPERM);
    }
    table.push(ControllingTtyEntry { sid, tty });
    Ok(())
}

/// Drop every session reference to a tty that is being hung up.
///
/// Linux `session_clear_tty()` (`drivers/tty/tty_jobctrl.c`) walks every task
/// in the owning session and calls `proc_clear_tty()`; the side table is keyed
/// by session id, so dropping the single entry is the same transition.
pub fn clear_controlling_tty(tty: ControllingTty) {
    CONTROLLING_TTYS.lock().retain(|entry| entry.tty != tty);
}

/// The session id that currently owns `tty`, i.e. Linux `tty->ctrl.session`.
pub fn controlling_tty_session(tty: ControllingTty) -> Option<i32> {
    CONTROLLING_TTYS
        .lock()
        .iter()
        .find(|entry| entry.tty == tty)
        .map(|entry| entry.sid)
}

/// `TIOCSCTTY` — `vendor/linux/drivers/tty/tty_jobctrl.c::tiocsctty()`.
///
/// ```c
/// if (current->signal->leader && task_session(current) == tty->ctrl.session)
///         goto unlock;                    /* already ours: success, no-op */
/// if (!current->signal->leader || current->signal->tty)
///         return -EPERM;
/// if (tty->ctrl.session) {
///         if (arg == 1 && capable(CAP_SYS_ADMIN))
///                 session_clear_tty(tty->ctrl.session);   /* steal it away */
///         else
///                 return -EPERM;
/// }
/// if ((file->f_mode & FMODE_READ) == 0 && !capable(CAP_SYS_ADMIN))
///         return -EPERM;
/// proc_set_tty(tty);
/// ```
///
/// The steal branch is what lets `agetty`/`login` (both root, both issuing
/// `ioctl(fd, TIOCSCTTY, 1)`) hand a terminal from the getty session to the
/// freshly `setsid()`-ed login session. Without it the second claim fails and
/// the new session ends up with no controlling terminal at all, which is
/// visible as `tty_nr == 0` in `/proc/<pid>/stat` and `TT ?` in `ps`.
pub fn tiocsctty(
    pid: i32,
    tty: ControllingTty,
    arg: i32,
    readable: bool,
    admin: bool,
) -> Result<(), i32> {
    let sid = session_id(pid).unwrap_or(pid);
    let leader = pid == sid;

    let mut table = CONTROLLING_TTYS.lock();
    let owned = table
        .iter()
        .find(|entry| entry.sid == sid)
        .map(|entry| entry.tty);
    if leader && owned == Some(tty) {
        return Ok(());
    }
    if !leader || owned.is_some() {
        return Err(EPERM);
    }
    if table.iter().any(|entry| entry.tty == tty) {
        if arg == 1 && admin {
            // `session_clear_tty()`: drop the previous owner's association.
            table.retain(|entry| entry.tty != tty);
        } else {
            return Err(EPERM);
        }
    }
    if !readable && !admin {
        return Err(EPERM);
    }
    table.push(ControllingTtyEntry { sid, tty });
    Ok(())
}

/// Remove the exiting/reaped task from the session side tables.
///
/// The controlling-TTY table is keyed only by the numeric session ID, so it
/// must not outlive the last task in that session. Otherwise a later task that
/// reuses the same PID as a session leader could inherit stale `/dev/tty`
/// access or block a fresh TTY claim.
pub fn release_task_session_state(pid: i32) {
    let sid = {
        let mut sessions = SESSIONS.lock();
        let sid = sessions
            .iter()
            .find(|entry| entry.pid == pid)
            .map(|entry| entry.sid)
            .unwrap_or(pid);
        sessions.retain(|entry| entry.pid != pid);
        let session_has_tasks = sessions.iter().any(|entry| entry.sid == sid);
        if session_has_tasks {
            return;
        }
        sid
    };

    CONTROLLING_TTYS.lock().retain(|entry| entry.sid != sid);
}

/// Linux `is_current_pgrp_orphaned()` (`kernel/pid.c`, called from
/// `__tty_check_change()`). A process group is orphaned unless some member
/// has a parent that is in the same session but a *different* group — i.e. a
/// job-control shell is still around to `SIGCONT` it after a stop. Orphaned
/// background groups get `EIO` instead of `SIGTTIN`/`SIGTTOU` + a stop, since
/// nothing would ever resume them.
pub fn pgrp_is_orphaned(pid: i32) -> bool {
    let pgrp = process_group(pid).unwrap_or(pid);
    let sid = session_id(pid).unwrap_or(pid);
    let mut has_anchor = false;
    fork::for_each_heap_task(|task| {
        if has_anchor || task.is_null() {
            return;
        }
        let member_pid = unsafe { (*task).pid };
        if process_group(member_pid) != Some(pgrp) {
            return;
        }
        let parent = unsafe { (*task).m26.real_parent };
        if parent.is_null() {
            return;
        }
        let parent_pid = unsafe { (*parent).pid };
        if process_group(parent_pid) != Some(pgrp) && session_id(parent_pid) == Some(sid) {
            has_anchor = true;
        }
    });
    !has_anchor
}

/// Inherit the parent's session and process group for a freshly forked child.
///
/// Linux keeps these IDs in the PID/session relationships copied by
/// `copy_process()`. Until Lupos grows full PIDTYPE_PGID/PIDTYPE_SID indexes,
/// the side table mirrors that inherited state explicitly.
pub fn inherit_from_parent(parent_pid: i32, child_pid: i32) {
    if parent_pid <= 0 || child_pid <= 0 {
        return;
    }
    let mut table = SESSIONS.lock();
    let parent = match table.iter().find(|entry| entry.pid == parent_pid).copied() {
        Some(entry) => entry,
        None => {
            let entry = SessionEntry {
                pid: parent_pid,
                pgid: parent_pid,
                sid: parent_pid,
            };
            table.push(entry);
            entry
        }
    };
    if let Some(entry) = table.iter_mut().find(|entry| entry.pid == child_pid) {
        entry.pgid = parent.pgid;
        entry.sid = parent.sid;
    } else {
        table.push(SessionEntry {
            pid: child_pid,
            pgid: parent.pgid,
            sid: parent.sid,
        });
    }
}

/// `setsid(2)` — create a new session and process group led by the caller.
///
/// Ref: `vendor/linux/kernel/sys.c::ksys_setsid()`. Linux rejects the call in
/// two cases before doing anything:
///
///   * the caller is already a session leader (`group_leader->signal->leader`);
///   * a process group id equal to the proposed session id already exists
///     (`pid_task(sid, PIDTYPE_PGID)`), i.e. the caller leads a process group.
///
/// On success it calls `proc_clear_tty()`. The controlling-tty side table is
/// keyed by session id, so moving to a brand-new session id (which by the
/// checks above cannot already own a tty) drops the association implicitly.
///
/// Both checks are made against the caller's *pre-existing* row. A pid with no
/// row at all has never joined a session or a group, so neither Linux
/// condition can hold for it; materialising a default row first (whose `sid`
/// and `pgid` both default to the pid) would make every such caller look like
/// a leader and wrongly fail.
pub unsafe fn sys_setsid() -> i64 {
    let pid = match current_pid() {
        Ok(pid) => pid,
        Err(errno) => return -(errno as i64),
    };
    let existing = {
        let table = SESSIONS.lock();
        table.iter().find(|entry| entry.pid == pid).copied()
    };
    // "Fail if I am already a session leader."
    if existing.is_some_and(|entry| entry.sid == pid) {
        return -(EPERM as i64);
    }
    // "Fail if a process group id already exists that equals the proposed
    // session id."
    if SESSIONS.lock().iter().any(|entry| entry.pgid == pid) {
        return -(EPERM as i64);
    }
    let entry = match update_entry(pid, |entry| {
        entry.sid = pid;
        entry.pgid = pid;
    }) {
        Ok(entry) => entry,
        Err(errno) => return -(errno as i64),
    };
    entry.sid as i64
}

/// `setpgid(2)` — assign `pid` to process group `pgid`.
pub unsafe fn sys_setpgid(pid: i32, pgid: i32) -> i64 {
    let caller = match current_pid() {
        Ok(pid) => pid,
        Err(errno) => return -(errno as i64),
    };
    let target = if pid == 0 { caller } else { pid };
    let group = if pgid == 0 { target } else { pgid };
    if target <= 0 || group < 0 {
        return -(EINVAL as i64);
    }
    if !task_exists(target) {
        return -(ESRCH as i64);
    }
    match update_entry(target, |entry| {
        entry.pgid = group;
        if entry.sid == 0 {
            entry.sid = caller;
        }
    }) {
        Ok(_) => 0,
        Err(errno) => -(errno as i64),
    }
}

#[cfg(test)]
pub fn reset_for_tests() {
    SESSIONS.lock().clear();
    CONTROLLING_TTYS.lock().clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn side_table_updates_group_and_session() {
        update_entry(10, |entry| {
            entry.sid = 10;
            entry.pgid = 10;
        })
        .unwrap();
        update_entry(11, |entry| {
            entry.sid = 10;
            entry.pgid = 10;
        })
        .unwrap();
        assert_eq!(session_id(11), Some(10));
        assert_eq!(process_group(11), Some(10));
    }

    #[test]
    fn forked_child_inherits_parent_group_and_session() {
        reset_for_tests();
        update_entry(20, |entry| {
            entry.sid = 20;
            entry.pgid = 21;
        })
        .unwrap();

        inherit_from_parent(20, 22);

        assert_eq!(session_id(22), Some(20));
        assert_eq!(process_group(22), Some(21));
    }

    #[test]
    fn releasing_last_session_task_clears_controlling_tty() {
        reset_for_tests();
        update_entry(30, |entry| {
            entry.sid = 30;
            entry.pgid = 30;
        })
        .unwrap();
        claim_controlling_tty(30, ControllingTty::Console(0x500)).unwrap();

        release_task_session_state(30);

        assert_eq!(controlling_tty(30), None);
        assert_eq!(
            claim_controlling_tty(40, ControllingTty::Console(0x500)),
            Ok(())
        );
    }

    #[test]
    fn releasing_one_member_preserves_live_session_tty() {
        reset_for_tests();
        update_entry(50, |entry| {
            entry.sid = 50;
            entry.pgid = 50;
        })
        .unwrap();
        inherit_from_parent(50, 51);
        claim_controlling_tty(50, ControllingTty::Unix98Pty(1, 99)).unwrap();

        release_task_session_state(51);

        assert_eq!(controlling_tty(50), Some(ControllingTty::Unix98Pty(1, 99)));
    }
    // ── vendor/linux/kernel/sys.c::ksys_setsid() parity ────────────────────

    fn with_current_pid<T>(pid: i32, f: impl FnOnce() -> T) -> T {
        let previous = unsafe { sched::get_current() };
        let mut task = alloc::boxed::Box::new(unsafe {
            core::mem::zeroed::<crate::kernel::task::TaskStruct>()
        });
        task.pid = pid;
        task.tgid = pid;
        unsafe { sched::set_current(&mut *task as *mut crate::kernel::task::TaskStruct) };
        let out = f();
        unsafe { sched::set_current(previous) };
        out
    }

    #[test]
    fn setsid_rejects_a_task_that_already_leads_its_session() {
        // "Fail if I am already a session leader" — ksys_setsid().
        reset_for_tests();
        update_entry(60, |entry| {
            entry.sid = 60;
            entry.pgid = 60;
        })
        .unwrap();

        let rc = with_current_pid(60, || unsafe { sys_setsid() });

        assert_eq!(rc, -(EPERM as i64));
        reset_for_tests();
    }

    #[test]
    fn setsid_rejects_a_process_group_leader() {
        // "Fail if a process group id already exists that equals the proposed
        // session id" — ksys_setsid(). Here pid 70 leads pgrp 70 inside
        // session 65, so `pid_task(sid, PIDTYPE_PGID)` is non-NULL.
        reset_for_tests();
        update_entry(65, |entry| {
            entry.sid = 65;
            entry.pgid = 65;
        })
        .unwrap();
        update_entry(70, |entry| {
            entry.sid = 65;
            entry.pgid = 70;
        })
        .unwrap();

        let rc = with_current_pid(70, || unsafe { sys_setsid() });

        assert_eq!(rc, -(EPERM as i64));
        reset_for_tests();
    }

    #[test]
    fn setsid_promotes_a_group_member_and_drops_the_controlling_tty() {
        // A plain member of session 65 / pgrp 65 becomes its own session and
        // group leader, and `proc_clear_tty()` leaves it without a /dev/tty.
        reset_for_tests();
        update_entry(65, |entry| {
            entry.sid = 65;
            entry.pgid = 65;
        })
        .unwrap();
        inherit_from_parent(65, 71);
        claim_controlling_tty(65, ControllingTty::Unix98Pty(3, 4)).unwrap();
        assert_eq!(controlling_tty(71), Some(ControllingTty::Unix98Pty(3, 4)));

        let rc = with_current_pid(71, || unsafe { sys_setsid() });

        assert_eq!(rc, 71);
        assert_eq!(session_id(71), Some(71));
        assert_eq!(process_group(71), Some(71));
        assert_eq!(controlling_tty(71), None);
        // The old session keeps its terminal, exactly as proc_clear_tty()
        // only touches the calling task's signal->tty.
        assert_eq!(controlling_tty(65), Some(ControllingTty::Unix98Pty(3, 4)));
        reset_for_tests();
    }

    #[test]
    fn controlling_tty_session_reports_the_owning_session() {
        reset_for_tests();
        update_entry(80, |entry| {
            entry.sid = 80;
            entry.pgid = 80;
        })
        .unwrap();
        claim_controlling_tty(80, ControllingTty::Unix98Pty(5, 6)).unwrap();

        assert_eq!(
            controlling_tty_session(ControllingTty::Unix98Pty(5, 6)),
            Some(80)
        );
        assert_eq!(
            controlling_tty_session(ControllingTty::Unix98Pty(7, 8)),
            None
        );
        reset_for_tests();
    }

    // ── vendor/linux/drivers/tty/tty_jobctrl.c::tiocsctty() parity ─────────

    #[test]
    fn tiocsctty_is_a_noop_when_the_leader_already_owns_the_tty() {
        reset_for_tests();
        update_entry(90, |entry| {
            entry.sid = 90;
            entry.pgid = 90;
        })
        .unwrap();
        let tty = ControllingTty::Console(0x440);
        assert_eq!(tiocsctty(90, tty, 1, true, true), Ok(()));

        assert_eq!(tiocsctty(90, tty, 0, true, false), Ok(()));
        assert_eq!(controlling_tty_session(tty), Some(90));
        reset_for_tests();
    }

    #[test]
    fn tiocsctty_rejects_a_non_session_leader() {
        reset_for_tests();
        update_entry(91, |entry| {
            entry.sid = 91;
            entry.pgid = 91;
        })
        .unwrap();
        inherit_from_parent(91, 92);

        assert_eq!(
            tiocsctty(92, ControllingTty::Console(0x440), 1, true, true),
            Err(EPERM)
        );
        reset_for_tests();
    }

    #[test]
    fn tiocsctty_rejects_a_leader_that_already_has_another_tty() {
        reset_for_tests();
        update_entry(93, |entry| {
            entry.sid = 93;
            entry.pgid = 93;
        })
        .unwrap();
        tiocsctty(93, ControllingTty::Console(0x440), 1, true, true).unwrap();

        assert_eq!(
            tiocsctty(93, ControllingTty::Unix98Pty(0, 1), 1, true, true),
            Err(EPERM)
        );
        reset_for_tests();
    }

    #[test]
    fn tiocsctty_steals_the_console_from_the_getty_session_for_root_login() {
        // agetty owns the console for session 100; `login` forks a child that
        // calls setsid() and then ioctl(0, TIOCSCTTY, 1) as root. Linux hands
        // the terminal over; before this parity fix the claim failed and the
        // login session was left with no controlling terminal, which surfaced
        // as tty_nr == 0 in /proc/<pid>/stat and `TT ?` in ps.
        reset_for_tests();
        let console = ControllingTty::Console(0x440);
        update_entry(100, |entry| {
            entry.sid = 100;
            entry.pgid = 100;
        })
        .unwrap();
        tiocsctty(100, console, 1, true, true).unwrap();
        update_entry(101, |entry| {
            entry.sid = 101;
            entry.pgid = 101;
        })
        .unwrap();

        assert_eq!(tiocsctty(101, console, 1, true, true), Ok(()));

        assert_eq!(controlling_tty_session(console), Some(101));
        assert_eq!(controlling_tty(101), Some(console));
        // session_clear_tty() dropped the previous owner's association.
        assert_eq!(controlling_tty(100), None);
        reset_for_tests();
    }

    #[test]
    fn tiocsctty_refuses_to_steal_without_arg_one_or_cap_sys_admin() {
        reset_for_tests();
        let console = ControllingTty::Console(0x440);
        update_entry(110, |entry| {
            entry.sid = 110;
            entry.pgid = 110;
        })
        .unwrap();
        tiocsctty(110, console, 1, true, true).unwrap();
        for pid in [111, 112] {
            update_entry(pid, |entry| {
                entry.sid = pid;
                entry.pgid = pid;
            })
            .unwrap();
        }

        // arg != 1, even with CAP_SYS_ADMIN.
        assert_eq!(tiocsctty(111, console, 0, true, true), Err(EPERM));
        // arg == 1 without CAP_SYS_ADMIN.
        assert_eq!(tiocsctty(112, console, 1, true, false), Err(EPERM));
        assert_eq!(controlling_tty_session(console), Some(110));
        reset_for_tests();
    }

    #[test]
    fn tiocsctty_rejects_a_write_only_fd_without_cap_sys_admin() {
        reset_for_tests();
        update_entry(120, |entry| {
            entry.sid = 120;
            entry.pgid = 120;
        })
        .unwrap();

        assert_eq!(
            tiocsctty(120, ControllingTty::Console(0x440), 1, false, false),
            Err(EPERM)
        );
        assert_eq!(
            tiocsctty(120, ControllingTty::Console(0x440), 1, false, true),
            Ok(())
        );
        reset_for_tests();
    }
}
