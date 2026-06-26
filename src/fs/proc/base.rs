//! linux-parity: partial
//! linux-source: vendor/linux/fs/proc/base.c
//! Process procfs base directory builders.
//!
//! Ref: `vendor/linux/fs/proc/base.c`

use alloc::sync::Arc;
use core::sync::atomic::Ordering;

use crate::fs::anon_inode::alloc_anon_file_with_kind;
use crate::fs::kernfs::{KernfsNode, add_child};
use crate::fs::ops::FileOps;
use crate::fs::types::{FileRef, InodeKind};
use crate::include::uapi::errno::ENOENT;
use crate::kernel::task::TaskStruct;
use crate::kernel::task::task_state::{EXIT_DEAD, EXIT_ZOMBIE};

static PROC_PID_STAT_FILE_OPS: FileOps = FileOps {
    name: "proc-pid-stat",
    read: Some(proc_pid_stat_read),
    write: None,
    llseek: None,
    fsync: None,
    poll: None,
    ioctl: None,
    mmap: None,
    release: None,
    readdir: None,
};

pub fn process_stat_file(pid: i32, flags: u32, mode: u32) -> Result<FileRef, i32> {
    if pid <= 0 || task_by_pid(pid).is_null() {
        return Err(ENOENT);
    }
    let _ = mode;
    let file = alloc_anon_file_with_kind(
        "stat",
        &PROC_PID_STAT_FILE_OPS,
        pid as usize,
        InodeKind::Regular,
        0o444,
    );
    file.flags.store(flags, Ordering::Release);
    Ok(file)
}

pub fn process_stat_file_from_proc_path(
    path: &str,
    flags: u32,
    mode: u32,
) -> Option<Result<FileRef, i32>> {
    let rest = path.strip_prefix("/proc/")?;
    let (pid_text, name) = rest.split_once('/')?;
    if name != "stat" {
        return None;
    }
    let Ok(pid) = pid_text.parse::<i32>() else {
        return None;
    };
    Some(process_stat_file(pid, flags, mode))
}

fn proc_pid_stat_read(file: &FileRef, buf: &mut [u8], pos: &mut u64) -> Result<usize, i32> {
    let pid = *file.private.lock() as i32;
    let task = task_by_pid(pid);
    if task.is_null() {
        return Err(ENOENT);
    }
    let text = unsafe { task_stat_text(task) };
    let start = (*pos as usize).min(text.len());
    let end = text.len().min(start + buf.len());
    let n = end.saturating_sub(start);
    buf[..n].copy_from_slice(&text.as_bytes()[start..end]);
    *pos += n as u64;
    Ok(n)
}

fn task_by_pid(pid: i32) -> *mut TaskStruct {
    let current = unsafe { crate::kernel::sched::get_current() };
    if !current.is_null() && unsafe { (*current).pid == pid } {
        return current;
    }
    crate::kernel::fork::find_heap_task_by_pid(pid)
}

unsafe fn task_stat_text(task: *mut TaskStruct) -> alloc::string::String {
    let (pid, ppid, state, comm) = unsafe {
        let parent = (*task).m26.real_parent;
        let ppid = if parent.is_null() { 0 } else { (*parent).pid };
        let exit_state = (*task).m26.exit_state | (*task).__state.load(Ordering::Acquire);
        let state = if exit_state & (EXIT_ZOMBIE | EXIT_DEAD) != 0 {
            'Z'
        } else {
            'R'
        };
        ((*task).pid, ppid, state, task_comm(task))
    };
    super::array::stat_text_with_ppid(pid, &comm, state, ppid)
}

unsafe fn task_comm(task: *mut TaskStruct) -> alloc::string::String {
    let bytes = unsafe { &(*task).comm };
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    match core::str::from_utf8(&bytes[..end]) {
        Ok("") | Err(_) => alloc::string::String::from("lupos"),
        Ok(comm) => alloc::string::String::from(comm),
    }
}

pub fn add_task_common(dir: &Arc<KernfsNode>) {
    add_child(
        dir,
        KernfsNode::new_file("stat", 0o444, Some(super::array::self_stat_show), None),
    );
    add_child(
        dir,
        KernfsNode::new_file("status", 0o444, Some(super::array::self_status_show), None),
    );
    add_child(
        dir,
        KernfsNode::new_file("statm", 0o444, Some(super::task_mmu::statm_show), None),
    );
    add_child(
        dir,
        KernfsNode::new_file("maps", 0o444, Some(super::task_mmu::maps_show), None),
    );
    add_child(
        dir,
        KernfsNode::new_file("smaps", 0o444, Some(super::task_mmu::smaps_show), None),
    );
    add_child(dir, KernfsNode::new_file("mem", 0o600, None, None));
    add_child(dir, KernfsNode::new_file("pagemap", 0o400, None, None));
    add_child(
        dir,
        KernfsNode::new_file(
            "cmdline",
            0o444,
            Some(super::array::self_cmdline_show),
            None,
        ),
    );
    // vendor/linux/fs/proc/base.c::proc_pid_environ_read
    add_child(
        dir,
        KernfsNode::new_file(
            "environ",
            0o400,
            Some(super::array::self_environ_show),
            None,
        ),
    );
    add_child(
        dir,
        KernfsNode::new_file("comm", 0o444, Some(super::array::self_comm_show), None),
    );
    add_child(
        dir,
        KernfsNode::new_file("cgroup", 0o444, Some(super::array::self_cgroup_show), None),
    );
    add_child(
        dir,
        KernfsNode::new_file(
            "oom_score",
            0o444,
            Some(super::array::self_oom_score_show),
            None,
        ),
    );
    add_child(
        dir,
        KernfsNode::new_file(
            "oom_score_adj",
            0o644,
            Some(super::array::self_oom_score_adj_show),
            Some(super::array::self_oom_score_adj_store),
        ),
    );
}
