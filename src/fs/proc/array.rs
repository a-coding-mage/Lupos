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
    kernel::task::task_state::{EXIT_DEAD, EXIT_ZOMBIE},
};

pub fn stat_text_with_ppid(pid: i32, comm: &str, state: char, ppid: i32) -> String {
    format!(
        "{} ({}) {} {} 0 0 0 -1 0 0 0 0 0 0 0 0 0 0 0 20 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0\n",
        pid, comm, state, ppid,
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
    let state = task_state_char(task);
    let comm = super::util::task_comm(task);
    super::util::copy_into(
        buf,
        &stat_text_with_ppid(unsafe { (*task).pid }, &comm, state, ppid),
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
    let parent = unsafe { (*task).m26.real_parent };
    if parent.is_null() {
        0
    } else {
        unsafe { (*parent).pid }
    }
}

fn task_state_char(task: *mut TaskStruct) -> char {
    if task.is_null() {
        return 'R';
    }
    let exit_state = unsafe {
        (*task).m26.exit_state | (*task).__state.load(core::sync::atomic::Ordering::Acquire)
    };
    if exit_state & (EXIT_ZOMBIE | EXIT_DEAD) != 0 {
        'Z'
    } else {
        'R'
    }
}

fn task_state_text(task: *mut TaskStruct) -> &'static str {
    if task_state_char(task) == 'Z' {
        "Z (zombie)"
    } else {
        "R (running)"
    }
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
