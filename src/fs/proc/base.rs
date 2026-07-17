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
use crate::include::uapi::errno::{EFAULT, EINVAL, ENOENT};
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

static PROC_PID_CGROUP_FILE_OPS: FileOps = FileOps {
    name: "proc-pid-cgroup",
    read: Some(proc_pid_cgroup_read),
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

pub fn process_cgroup_file(pid: i32, flags: u32, mode: u32) -> Result<FileRef, i32> {
    if pid <= 0 || task_by_pid(pid).is_null() {
        return Err(ENOENT);
    }
    let _ = mode;
    let file = alloc_anon_file_with_kind(
        "cgroup",
        &PROC_PID_CGROUP_FILE_OPS,
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

pub fn process_cgroup_file_from_proc_path(
    path: &str,
    flags: u32,
    mode: u32,
) -> Option<Result<FileRef, i32>> {
    let (pid, name) = parse_proc_pid_file(path)?;
    if name != "cgroup" {
        return None;
    }
    Some(process_cgroup_file(pid, flags, mode))
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

fn proc_pid_cgroup_read(file: &FileRef, buf: &mut [u8], pos: &mut u64) -> Result<usize, i32> {
    let pid = *file.private.lock() as i32;
    if task_by_pid(pid).is_null() {
        return Err(ENOENT);
    }
    let text = crate::kernel::cgroup::proc_cgroup_text_for_pid(pid);
    read_string_at(&text, buf, pos)
}

fn read_string_at(text: &str, buf: &mut [u8], pos: &mut u64) -> Result<usize, i32> {
    let start = (*pos as usize).min(text.len());
    let end = text.len().min(start + buf.len());
    let n = end.saturating_sub(start);
    buf[..n].copy_from_slice(&text.as_bytes()[start..end]);
    *pos += n as u64;
    Ok(n)
}

pub(crate) fn task_by_pid(pid: i32) -> *mut TaskStruct {
    let current = unsafe { crate::kernel::sched::get_current() };
    if !current.is_null() && unsafe { (*current).pid == pid } {
        return current;
    }
    let heap = crate::kernel::fork::find_heap_task_by_pid(pid);
    if !heap.is_null() {
        return heap;
    }
    crate::kernel::sched::find_pool_task_by_pid(pid)
}

fn parse_proc_pid_file(path: &str) -> Option<(i32, &str)> {
    if let Some(name) = path.strip_prefix("/proc/self/") {
        let task = unsafe { crate::kernel::sched::get_current() };
        let pid = if task.is_null() {
            1
        } else {
            unsafe { (*task).pid }
        };
        return Some((pid, name));
    }
    let rest = path.strip_prefix("/proc/")?;
    let (pid_text, name) = rest.split_once('/')?;
    let Ok(pid) = pid_text.parse::<i32>() else {
        return None;
    };
    Some((pid, name))
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
        ((*task).pid, ppid, state, super::util::task_comm(task))
    };
    super::array::stat_text_with_ppid(pid, &comm, state, ppid)
}

pub fn add_task_common(dir: &Arc<KernfsNode>) {
    add_child(
        dir,
        KernfsNode::new_file("stat", 0o444, Some(proc_pid_stat_show), None),
    );
    add_child(
        dir,
        KernfsNode::new_file("status", 0o444, Some(proc_pid_status_show), None),
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
        KernfsNode::new_file("cmdline", 0o444, Some(proc_pid_cmdline_show), None),
    );
    // vendor/linux/fs/proc/base.c::proc_pid_environ_read
    add_child(
        dir,
        KernfsNode::new_file("environ", 0o400, Some(proc_pid_environ_show), None),
    );
    add_child(
        dir,
        KernfsNode::new_file("comm", 0o444, Some(proc_pid_comm_show), None),
    );
    add_child(
        dir,
        KernfsNode::new_file("cgroup", 0o444, Some(proc_pid_cgroup_show), None),
    );
    add_child(
        dir,
        KernfsNode::new_file(
            "uid_map",
            0o644,
            Some(proc_pid_uid_map_show),
            Some(proc_pid_uid_map_store),
        ),
    );
    add_child(
        dir,
        KernfsNode::new_file(
            "gid_map",
            0o644,
            Some(proc_pid_gid_map_show),
            Some(proc_pid_gid_map_store),
        ),
    );
    add_child(
        dir,
        KernfsNode::new_file(
            "setgroups",
            0o644,
            Some(proc_pid_setgroups_show),
            Some(proc_pid_setgroups_store),
        ),
    );
    add_child(dir, super::namespaces::new_ns_dir());
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

fn proc_pid_from_node(node: &Arc<KernfsNode>) -> Result<i32, i32> {
    let parent = node.parent.lock().upgrade().ok_or(EINVAL)?;
    if parent.name == "self" {
        let task = unsafe { crate::kernel::sched::get_current() };
        return if task.is_null() {
            Err(ENOENT)
        } else {
            Ok(unsafe { (*task).pid })
        };
    }
    parent.name.parse::<i32>().map_err(|_| ENOENT)
}

fn proc_pid_stat_show(node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let pid = proc_pid_from_node(node)?;
    let task = task_by_pid(pid);
    if task.is_null() {
        return Err(ENOENT);
    }
    let text = unsafe { task_stat_text(task) };
    super::util::copy_into(buf, &text)
}

fn proc_pid_status_show(node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let pid = proc_pid_from_node(node)?;
    let task = task_by_pid(pid);
    if task.is_null() {
        return Err(ENOENT);
    }
    let (tgid, ppid, state, comm) = unsafe {
        let parent = (*task).m26.real_parent;
        let ppid = if parent.is_null() { 0 } else { (*parent).pid };
        let exit_state = (*task).m26.exit_state | (*task).__state.load(Ordering::Acquire);
        let state = if exit_state & (EXIT_ZOMBIE | EXIT_DEAD) != 0 {
            "Z (zombie)"
        } else {
            "R (running)"
        };
        ((*task).tgid, ppid, state, super::util::task_comm(task))
    };
    let text = super::util::format_status(&super::util::ProcStatusView {
        name: &comm,
        state,
        tgid,
        pid,
        ppid,
        locked_kb: super::util::task_locked_vm_kb(task),
        rss_anon_kb: super::util::task_rss_anon_kb(task),
        security: super::util::task_status_security(task),
    });
    super::util::copy_into(buf, &text)
}

fn proc_pid_comm_show(node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let pid = proc_pid_from_node(node)?;
    let task = task_by_pid(pid);
    if task.is_null() {
        return Err(ENOENT);
    }
    let text = alloc::format!("{}\n", super::util::task_comm(task));
    super::util::copy_into(buf, &text)
}

fn proc_pid_cmdline_show(node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let pid = proc_pid_from_node(node)?;
    let task = task_by_pid(pid);
    if task.is_null() {
        return Err(ENOENT);
    }
    let current = unsafe { crate::kernel::sched::get_current() };
    if task == current {
        return super::array::self_cmdline_show(node, buf);
    }
    let mm = unsafe { (*task).mm };
    if mm.is_null() {
        return Ok(0);
    }
    unsafe { read_task_mm_range(task, buf, (*mm).arg_start, (*mm).arg_end) }
}

fn proc_pid_environ_show(node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let pid = proc_pid_from_node(node)?;
    let task = task_by_pid(pid);
    if task.is_null() {
        return Err(ENOENT);
    }
    let current = unsafe { crate::kernel::sched::get_current() };
    if task == current {
        return super::array::self_environ_show(node, buf);
    }
    let mm = unsafe { (*task).mm };
    if mm.is_null() {
        return Ok(0);
    }
    unsafe { read_task_mm_range(task, buf, (*mm).env_start, (*mm).env_end) }
}

unsafe fn read_task_mm_range(
    task: *mut TaskStruct,
    buf: &mut [u8],
    start: u64,
    end: u64,
) -> Result<usize, i32> {
    if task.is_null() || unsafe { (*task).mm.is_null() } || start == 0 || end <= start {
        return Ok(0);
    }
    let len = (end - start).min(buf.len() as u64) as usize;
    let copied = crate::mm::mm_public::access_process_vm(
        unsafe { (*task).mm },
        start,
        buf.as_mut_ptr(),
        len,
        false,
    );
    if copied < 0 {
        Err(EFAULT)
    } else {
        Ok(copied as usize)
    }
}

fn proc_pid_cgroup_show(node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let pid = proc_pid_from_node(node)?;
    if task_by_pid(pid).is_null() {
        return Err(ENOENT);
    }
    super::util::copy_into(buf, &crate::kernel::cgroup::proc_cgroup_text_for_pid(pid))
}

fn task_user_namespace(
    node: &Arc<KernfsNode>,
) -> Result<*mut crate::kernel::user_namespace::UserNamespace, i32> {
    let task = task_by_pid(proc_pid_from_node(node)?);
    if task.is_null() {
        return Err(ENOENT);
    }
    let cred = unsafe { (*task).cred };
    if cred.is_null() || unsafe { (*cred).user_ns.is_null() } {
        return Ok(&raw const crate::kernel::user_namespace::INIT_USER_NS as *mut _);
    }
    Ok(unsafe { (*cred).user_ns as *mut crate::kernel::user_namespace::UserNamespace })
}

fn proc_pid_uid_map_show(node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let ns = task_user_namespace(node)?;
    super::util::copy_into(buf, &unsafe { (*ns).uid_map.render() })
}

fn proc_pid_gid_map_show(node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let ns = task_user_namespace(node)?;
    super::util::copy_into(buf, &unsafe { (*ns).gid_map.render() })
}

fn proc_pid_uid_map_store(node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    let ns = task_user_namespace(node)?;
    let text = core::str::from_utf8(buf).map_err(|_| EINVAL)?;
    let map = crate::kernel::user_namespace::UidGidMap::parse(text).map_err(i32::abs)?;
    if unsafe { (*ns).uid_map.nr_extents } != 0 {
        return Err(crate::include::uapi::errno::EPERM);
    }
    unsafe { (*ns).uid_map = map };
    Ok(buf.len())
}

fn proc_pid_gid_map_store(node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    let ns = task_user_namespace(node)?;
    let text = core::str::from_utf8(buf).map_err(|_| EINVAL)?;
    let map = crate::kernel::user_namespace::UidGidMap::parse(text).map_err(i32::abs)?;
    if unsafe { (*ns).gid_map.nr_extents } != 0 {
        return Err(crate::include::uapi::errno::EPERM);
    }
    unsafe { (*ns).gid_map = map };
    Ok(buf.len())
}

fn proc_pid_setgroups_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    super::util::copy_into(buf, "allow\n")
}

fn proc_pid_setgroups_store(_node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    if buf == b"deny\n" || buf == b"deny" {
        Ok(buf.len())
    } else {
        Err(EINVAL)
    }
}

#[cfg(test)]
mod tests {
    use crate::fs::kernfs::lookup;

    #[test]
    fn proc_pid_cmdline_uses_pid_specific_show_handler() {
        let source = include_str!("base.rs");
        let cmdline_entry = source
            .split("\"cmdline\"")
            .nth(1)
            .expect("cmdline proc entry must exist");
        assert!(
            cmdline_entry.contains("Some(proc_pid_cmdline_show)"),
            "/proc/<pid>/cmdline must not alias /proc/self/cmdline"
        );
    }

    #[test]
    fn task_common_exposes_namespace_directory() {
        let dir = crate::fs::kernfs::KernfsNode::new_dir("123", 0o555);
        super::add_task_common(&dir);
        let ns = lookup(&dir, "ns").expect("/proc/<pid>/ns");
        assert!(lookup(&ns, "user").is_some());
    }
}
