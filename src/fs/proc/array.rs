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
    super::util::copy_into(buf, &stat_text(1, "lupos", 'R'))
}

pub fn self_status_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let locked_kb = current_locked_vm_kb();
    let rss_anon_kb = current_rss_anon_kb();
    super::util::copy_into(
        buf,
        &format!(
            "Name:\tlupos\nState:\tR (running)\nTgid:\t1\nPid:\t1\nPPid:\t0\nUid:\t0\t0\t0\t0\nGid:\t0\t0\t0\t0\nVmLck:\t{:8} kB\nRssAnon:\t{:8} kB\n",
            locked_kb, rss_anon_kb
        ),
    )
}

fn current_locked_vm_kb() -> u64 {
    let current = unsafe { crate::kernel::sched::get_current() };
    if current.is_null() {
        return 0;
    }
    let mm = unsafe { (*current).mm };
    if mm.is_null() {
        return 0;
    }
    unsafe { (*mm).locked_vm.saturating_mul(4) }
}

fn current_rss_anon_kb() -> u64 {
    let current = unsafe { crate::kernel::sched::get_current() };
    if current.is_null() {
        return 0;
    }
    let mm = unsafe { (*current).mm };
    if mm.is_null() {
        return 0;
    }
    let mm = unsafe { &*mm };
    let mut kb = 0u64;
    for (_, _, entry) in mm.mm_mt.collect_entries() {
        let vma = unsafe { &*(entry as *const crate::mm::mm_types::VmAreaStruct) };
        if vma.vm_file != 0 || vma.vm_flags & crate::mm::vm_flags::VM_HUGEPAGE == 0 {
            continue;
        }
        if crate::mm::huge::thp_range_was_split(vma.vm_start, vma.vm_end) {
            continue;
        }
        kb = kb.saturating_add((vma.vm_end - vma.vm_start) / 1024);
    }
    kb
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
        let mut buf = [0u8; 256];
        let node = KernfsNode::new_file("status", 0o444, Some(self_status_show), None);
        let n = self_status_show(&node, &mut buf).unwrap();
        let text = core::str::from_utf8(&buf[..n]).unwrap();
        assert!(text.contains("VmLck:\t"));
    }
}
