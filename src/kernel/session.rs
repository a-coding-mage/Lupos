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
pub fn clear_controlling_tty(tty: ControllingTty) {
    CONTROLLING_TTYS.lock().retain(|entry| entry.tty != tty);
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
pub unsafe fn sys_setsid() -> i64 {
    let pid = match current_pid() {
        Ok(pid) => pid,
        Err(errno) => return -(errno as i64),
    };
    let cur = ensure_entry(pid);
    if cur.pgid == pid && cur.sid != pid {
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
}
