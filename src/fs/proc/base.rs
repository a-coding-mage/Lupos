//! linux-parity: partial
//! linux-source: vendor/linux/fs/proc/base.c
//! test-origin: linux:vendor/linux/fs/proc/base.c
//! Process procfs base directory builders.
//!
//! Ref: `vendor/linux/fs/proc/base.c`

use alloc::{string::String, sync::Arc};
use core::sync::atomic::Ordering;

use crate::fs::anon_inode::alloc_anon_file_with_kind;
use crate::fs::kernfs::{KernfsNode, add_child};
use crate::fs::ops::FileOps;
use crate::fs::types::{FileRef, InodeKind, InodeRef};
use crate::include::uapi::errno::{EACCES, EFAULT, EINVAL, ENOENT};
use crate::kernel::capability::{CAP_SYS_PTRACE, ns_capable};
use crate::kernel::cred::{Cred, INIT_CRED};
use crate::kernel::task::{
    TASK_COMM_LEN, TASK_CTRL_DUMPABLE_MASK, TASK_CTRL_DUMPABLE_SHIFT, TASK_CTRL_DUMPABLE_VALID,
    TaskStruct,
    task_state::{EXIT_DEAD, EXIT_ZOMBIE},
};

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

pub fn add_tgid_base(dir: &Arc<KernfsNode>) {
    add_task_common(dir);
    add_child(dir, new_task_dir());
}

pub fn add_task_common(dir: &Arc<KernfsNode>) {
    add_child(dir, super::fd::new_fd_dir());
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
        KernfsNode::new_file(
            "comm",
            0o644,
            Some(proc_pid_comm_show),
            Some(proc_pid_comm_store),
        ),
    );
    add_child(
        dir,
        KernfsNode::new_file("cgroup", 0o444, Some(proc_pid_cgroup_show), None),
    );
    add_child(
        dir,
        KernfsNode::new_dynamic_symlink("exe", proc_pid_exe_readlink),
    );
    // vendor/linux/fs/proc/base.c::proc_cwd_link / proc_root_link expose
    // the target task's retained fs_struct paths.  PipeWire relies on opening
    // /proc/<peer-pid>/root as a directory when determining sandbox status.
    add_child(
        dir,
        KernfsNode::new_dynamic_symlink("cwd", proc_pid_cwd_readlink),
    );
    add_child(
        dir,
        KernfsNode::new_dynamic_symlink("root", proc_pid_root_readlink),
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

fn new_task_dir() -> Arc<KernfsNode> {
    KernfsNode::new_dynamic_dir(
        "task",
        0o555,
        Some(proc_task_lookup),
        Some(proc_task_readdir),
    )
}

fn proc_task_lookup(dir: &InodeRef, name: &str) -> Result<InodeRef, i32> {
    let tid = parse_task_dir_tid(name)?;
    let leader = task_by_pid(proc_tgid_from_task_dir(dir)?);
    if leader.is_null() {
        return Err(ENOENT);
    }
    let task = task_by_pid(tid);
    if task.is_null() || unsafe { (*task).tgid != (*leader).tgid } {
        return Err(ENOENT);
    }
    let sb = dir.sb.lock().clone().ok_or(EINVAL)?;
    Ok(crate::fs::kernfs::inode_for_node(&sb, proc_tid_dir(tid)))
}

fn proc_task_readdir(file: &FileRef) -> Result<Option<(String, u64, InodeKind)>, i32> {
    if let Some(dot) = crate::fs::libfs::synthetic_readdir_dot_entry(file)? {
        return Ok(Some(dot));
    }
    let inode = file.inode().ok_or(EINVAL)?;
    let leader = task_by_pid(proc_tgid_from_task_dir(&inode)?);
    if leader.is_null() {
        return Err(ENOENT);
    }
    let tids = live_tids_for_tgid(unsafe { (*leader).tgid });
    let mut idx = file.pos.lock();
    let tid_idx = idx.saturating_sub(2) as usize;
    if tid_idx >= tids.len() {
        return Ok(None);
    }
    let tid = tids[tid_idx];
    *idx += 1;
    Ok(Some((
        alloc::format!("{}", tid),
        proc_tid_ino(tid),
        InodeKind::Directory,
    )))
}

fn proc_tgid_from_task_dir(dir: &InodeRef) -> Result<i32, i32> {
    let node = crate::fs::kernfs::node_from_inode(dir);
    let parent = node.parent.lock().upgrade().ok_or(EINVAL)?;
    if parent.name == "self" {
        let task = unsafe { crate::kernel::sched::get_current() };
        if task.is_null() {
            return Err(ENOENT);
        }
        return Ok(unsafe { (*task).tgid });
    }
    parent.name.parse::<i32>().map_err(|_| ENOENT)
}

fn parse_task_dir_tid(name: &str) -> Result<i32, i32> {
    if name.is_empty() || name.as_bytes().iter().any(|byte| !byte.is_ascii_digit()) {
        return Err(ENOENT);
    }
    let tid = name.parse::<i32>().map_err(|_| ENOENT)?;
    if tid <= 0 {
        return Err(ENOENT);
    }
    Ok(tid)
}

fn proc_tid_dir(tid: i32) -> Arc<KernfsNode> {
    let dir = KernfsNode::new_dir(&alloc::format!("{}", tid), 0o555);
    add_task_common(&dir);
    dir
}

fn live_tids_for_tgid(tgid: i32) -> alloc::vec::Vec<i32> {
    let mut tids = alloc::vec::Vec::new();
    push_live_tid_for_tgid(
        &mut tids,
        unsafe { crate::kernel::sched::get_current() },
        tgid,
    );
    crate::kernel::fork::for_each_heap_task(|task| push_live_tid_for_tgid(&mut tids, task, tgid));
    crate::kernel::sched::for_each_pool_task(|task| push_live_tid_for_tgid(&mut tids, task, tgid));
    tids.sort_unstable();
    tids.dedup();
    tids
}

fn push_live_tid_for_tgid(tids: &mut alloc::vec::Vec<i32>, task: *mut TaskStruct, tgid: i32) {
    if task.is_null() {
        return;
    }
    unsafe {
        if (*task).pid <= 0 || (*task).tgid != tgid || ((*task).m26.exit_state & EXIT_DEAD) != 0 {
            return;
        }
        tids.push((*task).pid);
    }
}

fn proc_tid_ino(tid: i32) -> u64 {
    0x7100_0000u64 + tid.max(0) as u64
}

fn proc_pid_exe_readlink(node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let pid = proc_pid_from_node(node)?;
    let task = task_by_pid(pid);
    if task.is_null() {
        return Err(ENOENT);
    }
    if !unsafe { proc_pid_ptrace_may_read_fscreds(task) } {
        return Err(EACCES);
    }
    let mm = unsafe { (*task).mm };
    let file = unsafe { crate::mm::mm_public::get_mm_exe_file_ref(mm) }.ok_or(ENOENT)?;
    let target = crate::fs::file::path_hint(&file)
        .or_else(|| crate::fs::mount::stable_path_for_dentry(&file.dentry))
        .unwrap_or_else(|| crate::fs::file::file_path(&file));
    let n = target.len().min(buf.len());
    buf[..n].copy_from_slice(&target.as_bytes()[..n]);
    crate::fs::file::fput(file);
    Ok(n)
}

unsafe fn proc_pid_ptrace_may_read_fscreds(target: *mut TaskStruct) -> bool {
    let current = unsafe { crate::kernel::sched::get_current() };
    if current.is_null() || target.is_null() {
        return false;
    }
    if current == target || unsafe { (*current).tgid == (*target).tgid } {
        return true;
    }

    let current_cred = unsafe { task_cred_or_init(current) };
    let target_cred = unsafe { task_cred_or_init(target) };
    if current_cred.is_null() || target_cred.is_null() {
        return false;
    }

    let has_ptrace_cap = unsafe { ns_capable((*target_cred).user_ns, CAP_SYS_PTRACE) };
    let ids_match = unsafe {
        let caller_uid = (*current_cred).fsuid;
        let caller_gid = (*current_cred).fsgid;
        caller_uid == (*target_cred).euid
            && caller_uid == (*target_cred).suid
            && caller_uid == (*target_cred).uid
            && caller_gid == (*target_cred).egid
            && caller_gid == (*target_cred).sgid
            && caller_gid == (*target_cred).gid
    };
    if !ids_match && !has_ptrace_cap {
        return false;
    }

    unsafe { proc_pid_task_still_dumpable(target) || has_ptrace_cap }
}

unsafe fn proc_pid_task_still_dumpable(task: *mut TaskStruct) -> bool {
    let control = unsafe { (*task).m27.mdwe_flags };
    if control & TASK_CTRL_DUMPABLE_VALID == 0 {
        return true;
    }
    ((control & TASK_CTRL_DUMPABLE_MASK) >> TASK_CTRL_DUMPABLE_SHIFT) == 1
}

unsafe fn task_cred_or_init(task: *mut TaskStruct) -> *const Cred {
    if task.is_null() {
        return &raw const INIT_CRED;
    }
    unsafe {
        if !(*task).cred.is_null() {
            (*task).cred
        } else if !(*task).m27.real_cred.is_null() {
            (*task).m27.real_cred
        } else {
            &raw const INIT_CRED
        }
    }
}

fn proc_pid_cwd_readlink(node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    proc_pid_fs_path_readlink(node, false, buf)
}

fn proc_pid_root_readlink(node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    proc_pid_fs_path_readlink(node, true, buf)
}

fn proc_pid_fs_path_readlink(
    node: &Arc<KernfsNode>,
    root: bool,
    buf: &mut [u8],
) -> Result<usize, i32> {
    let task = task_by_pid(proc_pid_from_node(node)?);
    if task.is_null() {
        return Err(ENOENT);
    }
    let fs = unsafe { crate::fs::fs_struct::task_fs(task) };
    if fs.is_null() {
        return Err(ENOENT);
    }
    let path = if root {
        unsafe { (*fs).root.lock().clone() }
    } else {
        unsafe { (*fs).pwd.lock().clone() }
    }
    .ok_or(ENOENT)?;
    let target = crate::fs::mount::namespace_path(&path)
        .or_else(|| crate::fs::mount::stable_path_for_dentry(&path.dentry))
        .ok_or(ENOENT)?;
    let n = target.len().min(buf.len());
    buf[..n].copy_from_slice(&target.as_bytes()[..n]);
    Ok(n)
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

fn proc_pid_comm_store(node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    let pid = proc_pid_from_node(node)?;
    let task = task_by_pid(pid);
    let current = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() || current.is_null() || unsafe { (*task).tgid != (*current).tgid } {
        return Err(EINVAL);
    }

    let copy = buf.len().min(TASK_COMM_LEN - 1);
    let mut comm = [0u8; TASK_COMM_LEN];
    comm[..copy].copy_from_slice(&buf[..copy]);
    unsafe {
        (*task).comm = comm;
    }
    Ok(buf.len())
}

pub fn proc_comm_write_allowed(inode: &InodeRef) -> bool {
    if !core::ptr::eq(inode.fops, &crate::fs::kernfs::KERNFS_FILE_FILE_OPS) {
        return false;
    }
    let node = crate::fs::kernfs::node_from_inode(inode);
    if node.name != "comm" {
        return false;
    }
    let Ok(pid) = proc_pid_from_node(&node) else {
        return false;
    };
    let task = task_by_pid(pid);
    let current = unsafe { crate::kernel::sched::get_current() };
    !task.is_null() && !current.is_null() && unsafe { (*task).tgid == (*current).tgid }
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
    use alloc::boxed::Box;
    use core::sync::atomic::AtomicUsize;

    use crate::kernel::capability::{CAP_SYS_PTRACE, KernelCapT};
    use crate::kernel::cred::{GroupInfo, KGid, KUid};
    use crate::kernel::sched;

    use super::*;
    use crate::fs::kernfs::{inode_for_node, lookup};
    use crate::fs::ops::NOOP_SUPER_OPS;
    use crate::fs::types::SuperBlock;

    fn test_cred(uid: u32, gid: u32, fsuid: u32, fsgid: u32, caps: KernelCapT) -> Box<Cred> {
        Box::new(Cred {
            usage: AtomicUsize::new(1),
            uid: KUid(uid),
            gid: KGid(gid),
            suid: KUid(uid),
            sgid: KGid(gid),
            euid: KUid(uid),
            egid: KGid(gid),
            fsuid: KUid(fsuid),
            fsgid: KGid(fsgid),
            cap_inheritable: KernelCapT::empty(),
            cap_permitted: caps,
            cap_effective: caps,
            cap_bset: KernelCapT::full(),
            cap_ambient: KernelCapT::empty(),
            securebits: 0,
            group_info: GroupInfo::default(),
            user_ns: core::ptr::null(),
        })
    }

    fn test_task(pid: i32, tgid: i32, cred: &Cred) -> Box<TaskStruct> {
        let mut task = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        task.pid = pid;
        task.tgid = tgid;
        task.cred = cred as *const Cred;
        task.m27.real_cred = cred as *const Cred;
        task
    }

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

    #[test]
    fn task_common_exposes_vendor_fs_magic_links() {
        let dir = crate::fs::kernfs::KernfsNode::new_dir("123", 0o555);
        super::add_task_common(&dir);
        for name in ["cwd", "root", "exe"] {
            let node = lookup(&dir, name).unwrap_or_else(|| panic!("/proc/<pid>/{name}"));
            assert_eq!(
                node.mode & crate::include::uapi::stat::S_IFMT,
                crate::include::uapi::stat::S_IFLNK
            );
        }

        let vendor = include_str!("../../../vendor/linux/fs/proc/base.c");
        assert!(vendor.contains("LNK(\"cwd\",        proc_cwd_link)"));
        assert!(vendor.contains("LNK(\"root\",       proc_root_link)"));
    }

    #[test]
    fn tgid_base_exposes_linux_task_and_fd_directories() {
        let dir = crate::fs::kernfs::KernfsNode::new_dir("123", 0o555);
        super::add_tgid_base(&dir);
        assert!(lookup(&dir, "fd").is_some());
        assert!(lookup(&dir, "task").is_some());

        let vendor = include_str!("../../../vendor/linux/fs/proc/base.c");
        assert!(vendor.contains("DIR(\"task\",       S_IRUGO|S_IXUGO"));
        assert!(vendor.contains("DIR(\"fd\",         S_IRUSR|S_IXUSR"));
    }

    #[test]
    fn proc_task_lookup_exposes_current_thread_comm() {
        let previous = unsafe { sched::get_current() };
        let cred = test_cred(1000, 1000, 1000, 1000, KernelCapT::empty());
        let mut current = test_task(3210, 3210, &cred);

        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);
            let sb = SuperBlock::alloc("proc", 0x9fa0, &NOOP_SUPER_OPS);
            let self_dir = crate::fs::kernfs::KernfsNode::new_dir("self", 0o555);
            super::add_tgid_base(&self_dir);
            let task_node = lookup(&self_dir, "task").expect("/proc/self/task");
            let task_inode = inode_for_node(&sb, task_node);
            let lookup_task = task_inode.ops.lookup.expect("task lookup");
            let tid_inode = lookup_task(&task_inode, "3210").expect("/proc/self/task/3210");
            let tid_node = crate::fs::kernfs::node_from_inode(&tid_inode);
            let comm = lookup(&tid_node, "comm").expect("tid comm");

            proc_pid_comm_store(&comm, b"wp-data-loop").expect("comm write");
            assert_eq!(
                crate::fs::proc::util::task_comm(&mut *current as *mut TaskStruct),
                "wp-data-loop"
            );

            sched::set_current(previous);
        }
    }

    #[test]
    fn proc_pid_exe_ptrace_read_fscreds_uses_fs_ids() {
        let previous = unsafe { sched::get_current() };
        let current_cred = test_cred(1000, 1000, 2000, 3000, KernelCapT::empty());
        let target_cred = test_cred(2000, 3000, 2000, 3000, KernelCapT::empty());
        let mut current = test_task(10, 10, &current_cred);
        let mut target = test_task(20, 20, &target_cred);

        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);
            assert!(proc_pid_ptrace_may_read_fscreds(
                &mut *target as *mut TaskStruct
            ));
            sched::set_current(previous);
        }
    }

    #[test]
    fn proc_pid_exe_ptrace_read_fscreds_rejects_cross_ids() {
        let previous = unsafe { sched::get_current() };
        let current_cred = test_cred(1000, 1000, 1000, 1000, KernelCapT::empty());
        let target_cred = test_cred(2000, 3000, 2000, 3000, KernelCapT::empty());
        let mut current = test_task(10, 10, &current_cred);
        let mut target = test_task(20, 20, &target_cred);

        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);
            assert!(!proc_pid_ptrace_may_read_fscreds(
                &mut *target as *mut TaskStruct
            ));
            sched::set_current(previous);
        }
    }

    #[test]
    fn proc_pid_exe_ptrace_read_fscreds_rechecks_dumpability() {
        let previous = unsafe { sched::get_current() };
        let current_cred = test_cred(1000, 1000, 1000, 1000, KernelCapT::empty());
        let target_cred = test_cred(1000, 1000, 1000, 1000, KernelCapT::empty());
        let mut current = test_task(10, 10, &current_cred);
        let mut target = test_task(20, 20, &target_cred);
        target.m27.mdwe_flags = TASK_CTRL_DUMPABLE_VALID;

        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);
            assert!(!proc_pid_ptrace_may_read_fscreds(
                &mut *target as *mut TaskStruct
            ));

            let mut caps = KernelCapT::empty();
            caps.raise(CAP_SYS_PTRACE);
            let privileged_cred = test_cred(3000, 3000, 3000, 3000, caps);
            let mut privileged = test_task(30, 30, &privileged_cred);
            sched::set_current(&mut *privileged as *mut TaskStruct);
            assert!(proc_pid_ptrace_may_read_fscreds(
                &mut *target as *mut TaskStruct
            ));
            sched::set_current(previous);
        }
    }
}
