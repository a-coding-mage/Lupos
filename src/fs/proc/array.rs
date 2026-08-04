//! linux-parity: partial
//! linux-source: vendor/linux/fs/proc/array.c
//! test-origin: linux:vendor/linux/fs/proc/array.c
//! Process status array formatting.
//!
//! Ref: `vendor/linux/fs/proc/array.c`

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::sync::Arc;

use crate::fs::kernfs::KernfsNode;
use crate::include::uapi::errno::EINVAL;
use crate::mm::oom::{OOM_SCORE_ADJ_MAX, OOM_SCORE_ADJ_MIN};
use crate::{
    kernel::task::TaskStruct,
    kernel::task::task_state::{
        __TASK_STOPPED, __TASK_TRACED, EXIT_DEAD, EXIT_ZOMBIE, TASK_INTERRUPTIBLE, TASK_NOLOAD,
        TASK_PARKED, TASK_RUNNING, TASK_UNINTERRUPTIBLE,
    },
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcStatIds {
    pub ppid: i32,
    pub pgrp: i32,
    pub session: i32,
    pub tty_nr: i32,
    pub tty_pgrp: i32,
}

pub fn stat_text_with_ids(pid: i32, comm: &str, state: char, ids: ProcStatIds) -> String {
    format!(
        "{} ({}) {} {} {} {} {} {} 0 0 0 0 0 0 0 0 0 0 0 20 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0\n",
        pid, comm, state, ids.ppid, ids.pgrp, ids.session, ids.tty_nr, ids.tty_pgrp,
    )
}

pub fn stat_text_with_ppid(pid: i32, comm: &str, state: char, ppid: i32) -> String {
    stat_text_with_ids(
        pid,
        comm,
        state,
        ProcStatIds {
            ppid,
            pgrp: pid,
            session: pid,
            tty_nr: 0,
            tty_pgrp: -1,
        },
    )
}

pub fn stat_text(pid: i32, comm: &str, state: char) -> String {
    stat_text_with_ppid(pid, comm, state, 0)
}

pub fn self_stat_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        return super::util::copy_into(buf, &stat_text(1, "lupos", 'R'));
    }
    let ppid = task_ppid(task);
    let ids = task_stat_ids(task, ppid);
    let state = task_state_char(task);
    let comm = super::util::task_comm(task);
    super::util::copy_into(
        buf,
        &stat_text_with_ids(unsafe { (*task).pid }, &comm, state, ids),
    )
}

pub fn self_status_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        return super::util::copy_into(
            buf,
            &super::util::format_status(&super::util::ProcStatusView {
                name: "lupos",
                state: "R (running)",
                tgid: 1,
                pid: 1,
                ppid: 0,
                locked_kb: 0,
                rss_anon_kb: 0,
                security: super::util::task_status_security(task),
            }),
        );
    }
    let comm = super::util::task_comm(task);
    let text = super::util::format_status(&super::util::ProcStatusView {
        name: &comm,
        state: task_state_text(task),
        tgid: unsafe { (*task).tgid },
        pid: unsafe { (*task).pid },
        ppid: task_ppid(task),
        locked_kb: super::util::task_locked_vm_kb(task),
        rss_anon_kb: super::util::task_rss_anon_kb(task),
        security: super::util::task_status_security(task),
    });
    super::util::copy_into(buf, &text)
}

fn task_ppid(task: *mut TaskStruct) -> i32 {
    if task.is_null() {
        return 0;
    }
    crate::kernel::pid_namespace::task_ppid_vnr(task)
}

pub fn task_stat_ids_for_pid(pid: i32, ppid: i32) -> ProcStatIds {
    let pgrp = crate::kernel::session::process_group(pid).unwrap_or(pid);
    let session = crate::kernel::session::session_id(pid).unwrap_or(pid);
    let (tty_nr, tty_pgrp) = task_tty_stat(pid);
    ProcStatIds {
        ppid,
        pgrp,
        session,
        tty_nr,
        tty_pgrp,
    }
}

fn task_stat_ids(task: *mut TaskStruct, ppid: i32) -> ProcStatIds {
    if task.is_null() {
        return ProcStatIds {
            ppid,
            pgrp: 0,
            session: 0,
            tty_nr: 0,
            tty_pgrp: -1,
        };
    }
    task_stat_ids_for_pid(unsafe { (*task).pid }, ppid)
}

fn task_tty_stat(pid: i32) -> (i32, i32) {
    match crate::kernel::session::controlling_tty(pid) {
        Some(crate::kernel::session::ControllingTty::Console(rdev)) => {
            let pgrp = crate::linux_driver_abi::tty::compat_tty_foreground_pgrp().unwrap_or(-1);
            (rdev as i32, pgrp)
        }
        Some(crate::kernel::session::ControllingTty::Unix98Pty(index, token)) => {
            let dev = crate::init::noinitramfs::new_encode_dev(crate::init::noinitramfs::mkdev(
                crate::linux_driver_abi::tty::pty::UNIX98_PTY_SLAVE_MAJOR,
                index,
            ));
            let pgrp =
                crate::linux_driver_abi::tty::pty::foreground_pgrp(index, token).unwrap_or(-1);
            (dev as i32, pgrp)
        }
        None => (0, -1),
    }
}

/// Linux `include/linux/sched.h:TASK_REPORT`.
const TASK_REPORT: u32 = TASK_RUNNING
    | TASK_INTERRUPTIBLE
    | TASK_UNINTERRUPTIBLE
    | __TASK_STOPPED
    | __TASK_TRACED
    | EXIT_DEAD
    | EXIT_ZOMBIE
    | TASK_PARKED;

/// Linux `include/linux/sched.h:TASK_REPORT_IDLE`.
const TASK_REPORT_IDLE: u32 = TASK_REPORT + 1;

/// Linux `include/linux/sched.h:TASK_IDLE`.
const TASK_IDLE: u32 = TASK_UNINTERRUPTIBLE | TASK_NOLOAD;

/// Linux `fs/proc/array.c:task_state_array`.
const TASK_STATE_ARRAY: [&str; 9] = [
    "R (running)",
    "S (sleeping)",
    "D (disk sleep)",
    "T (stopped)",
    "t (tracing stop)",
    "X (dead)",
    "Z (zombie)",
    "P (parked)",
    "I (idle)",
];

/// Linux `include/linux/sched.h:task_index_to_char()`.
const TASK_STATE_CHARS: [u8; 9] = *b"RSDTtXZPI";

/// Linux `include/linux/sched.h:__task_state_index()`.
///
/// `fls()` on the `TASK_REPORT`-masked state selects the reporting slot;
/// `TASK_RUNNING` is zero, so `fls(0) == 0` yields the running entry.
///
/// Lupos has no `TASK_RTLOCK_WAIT` or `TASK_FROZEN` state yet. Linux folds
/// both into `TASK_UNINTERRUPTIBLE` here; once those states exist they must be
/// added to this function rather than to the callers.
fn task_state_index(tsk_state: u32, tsk_exit_state: u32) -> usize {
    let mut state = (tsk_state | tsk_exit_state) & TASK_REPORT;
    if tsk_state & TASK_IDLE == TASK_IDLE {
        state = TASK_REPORT_IDLE;
    }
    // fls(): 1-based index of the most significant set bit, 0 when no bit set.
    (u32::BITS - state.leading_zeros()) as usize
}

fn task_report_index(task: *mut TaskStruct) -> usize {
    if task.is_null() {
        return 0;
    }
    let (state, exit_state) = unsafe {
        (
            (*task).__state.load(core::sync::atomic::Ordering::Acquire),
            (*task).m26.exit_state,
        )
    };
    task_state_index(state, exit_state).min(TASK_STATE_ARRAY.len() - 1)
}

pub(super) fn task_state_char(task: *mut TaskStruct) -> char {
    TASK_STATE_CHARS[task_report_index(task)] as char
}

pub(super) fn task_state_text(task: *mut TaskStruct) -> &'static str {
    TASK_STATE_ARRAY[task_report_index(task)]
}

pub fn self_comm_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    super::util::copy_into(buf, "lupos\n")
}

/// Read /proc/self/cmdline — null-separated argv strings from mm->arg_start..arg_end.
/// Ref: vendor/linux/fs/proc/base.c::proc_pid_cmdline_read
pub fn self_cmdline_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    if let Some(n) = read_mm_range(buf, |mm| {
        let start = unsafe { (*mm).arg_start };
        let end = unsafe { (*mm).arg_end };
        (start, end)
    }) {
        return Ok(n);
    }
    // Fallback when mm is unavailable (e.g. kernel threads).
    super::util::copy_into(buf, "lupos\0")
}

/// Read /proc/self/environ — null-separated KEY=VALUE strings from mm->env_start..env_end.
/// Ref: vendor/linux/fs/proc/base.c::proc_pid_environ_read
pub fn self_environ_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    if let Some(n) = read_mm_range(buf, |mm| {
        let start = unsafe { (*mm).env_start };
        let end = unsafe { (*mm).env_end };
        (start, end)
    }) {
        return Ok(n);
    }
    Ok(0)
}

/// Read a byte range from the current process's user virtual address space,
/// bounded by [start, end) obtained from mm_struct via `range_fn`.
/// Returns None when the task or mm is unavailable.
fn read_mm_range(
    buf: &mut [u8],
    range_fn: impl Fn(*mut crate::mm::mm_types::MmStruct) -> (u64, u64),
) -> Option<usize> {
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        return None;
    }
    let mm = unsafe { (*task).mm };
    if mm.is_null() {
        return None;
    }
    let (start, end) = range_fn(mm);
    if start == 0 || end <= start {
        return Some(0);
    }
    let len = (end - start) as usize;
    let to_copy = len.min(buf.len());
    // Safety: start..start+to_copy is in the current process's address space;
    // copy_from_user handles page faults safely.
    let unfilled = unsafe {
        crate::arch::x86::kernel::uaccess::copy_from_user(
            buf.as_mut_ptr(),
            start as *const u8,
            to_copy,
        )
    };
    Some(to_copy - unfilled)
}

pub fn self_cgroup_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let task = unsafe { crate::kernel::sched::get_current() };
    let pid = if task.is_null() {
        1
    } else {
        unsafe { (*task).pid }
    };
    super::util::copy_into(buf, &crate::kernel::cgroup::proc_cgroup_text_for_pid(pid))
}

pub fn self_oom_score_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    super::util::copy_into(buf, "0\n")
}

pub fn self_oom_score_adj_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    super::util::copy_into(buf, "0\n")
}

pub fn self_oom_score_adj_store(_node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    let text = core::str::from_utf8(buf).map_err(|_| EINVAL)?;
    let value = text.trim().parse::<i16>().map_err(|_| EINVAL)?;
    if !(OOM_SCORE_ADJ_MIN..=OOM_SCORE_ADJ_MAX).contains(&value) {
        return Err(EINVAL);
    }
    Ok(buf.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stat_text_reports_ppid_in_fourth_field() {
        let text = stat_text_with_ppid(425, "executor", 'R', 424);
        let mut parts = text.split_whitespace();
        assert_eq!(parts.next(), Some("425"));
        assert_eq!(parts.next(), Some("(executor)"));
        assert_eq!(parts.next(), Some("R"));
        assert_eq!(parts.next(), Some("424"));
    }

    #[test]
    fn stat_text_reports_linux_session_and_tty_fields() {
        let text = stat_text_with_ids(
            701,
            "xfce4-session",
            'S',
            ProcStatIds {
                ppid: 700,
                pgrp: 701,
                session: 701,
                tty_nr: crate::init::noinitramfs::new_encode_dev(crate::init::noinitramfs::mkdev(
                    crate::linux_driver_abi::tty::pty::UNIX98_PTY_SLAVE_MAJOR,
                    3,
                )) as i32,
                tty_pgrp: 701,
            },
        );
        let fields: alloc::vec::Vec<&str> = text.split_whitespace().collect();

        assert_eq!(fields[3], "700");
        assert_eq!(fields[4], "701");
        assert_eq!(fields[5], "701");
        assert_eq!(fields[6], "34819");
        assert_eq!(fields[7], "701");
    }

    #[test]
    fn stat_ids_report_controlling_console_tty() {
        crate::kernel::session::reset_for_tests();
        crate::linux_driver_abi::tty::reset_compat_tty_state();

        let rdev = crate::init::noinitramfs::new_encode_dev(crate::init::noinitramfs::mkdev(4, 7));
        crate::kernel::session::claim_controlling_tty(
            701,
            crate::kernel::session::ControllingTty::Console(rdev as u64),
        )
        .unwrap();
        crate::linux_driver_abi::tty::tty_ioctl_compat(
            crate::linux_driver_abi::tty::TIOCSCTTY,
            701,
        )
        .unwrap();

        let ids = task_stat_ids_for_pid(701, 700);

        assert_eq!(ids.ppid, 700);
        assert_eq!(ids.pgrp, 701);
        assert_eq!(ids.session, 701);
        assert_eq!(ids.tty_nr, rdev as i32);
        assert_eq!(ids.tty_pgrp, 701);

        crate::kernel::session::reset_for_tests();
        crate::linux_driver_abi::tty::reset_compat_tty_state();
    }

    #[test]
    fn status_includes_linux_vmlck_field() {
        let mut buf = [0u8; 512];
        let node = KernfsNode::new_file("status", 0o444, Some(self_status_show), None);
        let n = self_status_show(&node, &mut buf).unwrap();
        let text = core::str::from_utf8(&buf[..n]).unwrap();
        assert!(text.contains("VmLck:\t"));
        assert!(text.contains("CapEff:\t"));
        assert!(text.contains("NoNewPrivs:\t"));
    }
}
