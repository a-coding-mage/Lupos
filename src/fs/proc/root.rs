//! linux-parity: partial
//! linux-source: vendor/linux/fs/proc/root.c
//! procfs root directory population.
//!
//! Ref: `vendor/linux/fs/proc/root.c`

use alloc::{format, string::String, sync::Arc, vec::Vec};

use crate::fs::kernfs::{KernfsNode, add_child};
use crate::fs::types::{FileRef, InodeKind, InodeRef};
use crate::include::uapi::errno::{EINVAL, ENOENT};
use crate::kernel::task::TaskStruct;
use crate::kernel::task::task_state::EXIT_DEAD;

pub fn new_root() -> Arc<KernfsNode> {
    KernfsNode::new_dynamic_dir("/", 0o555, Some(proc_root_lookup), Some(proc_root_readdir))
}

pub fn filesystems_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    super::util::copy_into(buf, &crate::fs::filesystems::render_filesystems())
}

pub fn mounts_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    super::util::copy_into(buf, &crate::fs::proc_namespace::render_mounts())
}

pub fn mountinfo_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    super::util::copy_into(buf, &crate::fs::proc_namespace::render_mountinfo())
}

pub fn mountstats_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    super::util::copy_into(buf, &crate::fs::proc_namespace::render_mountstats())
}

pub fn lupos_boot_trace_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    super::util::copy_into(buf, &crate::init::boot_trace::render())
}

pub fn populate_root(root: &Arc<KernfsNode>) {
    for (name, show) in [
        ("version", super::version::show as super::util::ProcShow),
        ("uptime", super::uptime::show as super::util::ProcShow),
        ("loadavg", super::loadavg::show as super::util::ProcShow),
        ("meminfo", super::meminfo::show as super::util::ProcShow),
        ("stat", super::stat::show as super::util::ProcShow),
        ("cpuinfo", super::cpuinfo::show as super::util::ProcShow),
        ("cmdline", super::cmdline::show as super::util::ProcShow),
        ("filesystems", filesystems_show as super::util::ProcShow),
        ("mounts", mounts_show as super::util::ProcShow),
        ("mountinfo", mountinfo_show as super::util::ProcShow),
        ("mountstats", mountstats_show as super::util::ProcShow),
        ("swaps", super::swaps::show as super::util::ProcShow),
        ("devices", super::devices::show as super::util::ProcShow),
        (
            "interrupts",
            super::interrupts::show as super::util::ProcShow,
        ),
        ("softirqs", super::softirqs::show as super::util::ProcShow),
        ("consoles", super::consoles::show as super::util::ProcShow),
        (
            "bootconfig",
            super::bootconfig::show as super::util::ProcShow,
        ),
        ("kcore", super::kcore::show as super::util::ProcShow),
        ("kmsg", super::kmsg::show as super::util::ProcShow),
        ("vmcore", super::vmcore::show as super::util::ProcShow),
        (
            "pagetypeinfo",
            super::page::pagetypeinfo_show as super::util::ProcShow,
        ),
        (
            "kpageflags",
            super::page::pagetypeinfo_show as super::util::ProcShow,
        ),
        (
            "lupos_boot_trace",
            lupos_boot_trace_show as super::util::ProcShow,
        ),
    ] {
        add_child(root, KernfsNode::new_file(name, 0o444, Some(show), None));
    }
    add_child(root, super::self_::new_self_dir());
    add_child(root, super::thread_self::new_thread_self());
    add_child(root, super::proc_net::new_net_dir());
    add_child(root, super::proc_sysctl::new_sys_dir());
    add_child(root, super::proc_tty::new_tty_dir());
}

fn proc_root_lookup(dir: &InodeRef, name: &str) -> Result<InodeRef, i32> {
    let pid = parse_pid_name(name)?;
    if super::base::task_by_pid(pid).is_null() {
        return Err(ENOENT);
    }
    let sb = dir.sb.lock().clone().ok_or(EINVAL)?;
    Ok(crate::fs::kernfs::inode_for_node(&sb, proc_pid_dir(pid)))
}

fn proc_root_readdir(file: &FileRef) -> Result<Option<(String, u64, InodeKind)>, i32> {
    if let Some(dot) = crate::fs::libfs::synthetic_readdir_dot_entry(file)? {
        return Ok(Some(dot));
    }

    let inode = file.inode().ok_or(EINVAL)?;
    let node = crate::fs::kernfs::node_from_inode(&inode);
    let static_entries = crate::fs::kernfs::list(&node);
    let pids = live_pids();

    let mut idx = file.pos.lock();
    let entry_idx = idx.saturating_sub(2) as usize;
    if entry_idx < static_entries.len() {
        let entry = static_entries[entry_idx].clone();
        *idx += 1;
        return Ok(Some(entry));
    }

    let pid_idx = entry_idx.saturating_sub(static_entries.len());
    if pid_idx >= pids.len() {
        return Ok(None);
    }
    let pid = pids[pid_idx];
    *idx += 1;
    Ok(Some((
        format!("{}", pid),
        proc_pid_ino(pid),
        InodeKind::Directory,
    )))
}

fn proc_pid_dir(pid: i32) -> Arc<KernfsNode> {
    let dir = KernfsNode::new_dir(&format!("{}", pid), 0o555);
    super::base::add_task_common(&dir);
    dir
}

fn parse_pid_name(name: &str) -> Result<i32, i32> {
    if name.is_empty() || name.as_bytes().iter().any(|byte| !byte.is_ascii_digit()) {
        return Err(ENOENT);
    }
    let pid = name.parse::<i32>().map_err(|_| ENOENT)?;
    if pid <= 0 {
        return Err(ENOENT);
    }
    Ok(pid)
}

fn live_pids() -> Vec<i32> {
    let mut pids = Vec::new();
    let current = unsafe { crate::kernel::sched::get_current() };
    push_live_pid(&mut pids, current);
    crate::kernel::fork::for_each_heap_task(|task| push_live_pid(&mut pids, task));
    crate::kernel::sched::for_each_pool_task(|task| push_live_pid(&mut pids, task));
    pids.sort_unstable();
    pids.dedup();
    pids
}

fn push_live_pid(pids: &mut Vec<i32>, task: *mut TaskStruct) {
    if task.is_null() {
        return;
    }
    unsafe {
        // Linux exposes only thread-group leaders while iterating `/proc`.
        // A non-leader TID remains addressable explicitly as `/proc/<tid>`
        // (and under `/proc/<tgid>/task/<tid>`), but it must not appear in
        // getdents(2).  Process managers rely on this distinction and would
        // otherwise treat every pthread as a separate process.
        if (*task).pid <= 0
            || (*task).pid != (*task).tgid
            || ((*task).m26.exit_state & EXIT_DEAD) != 0
        {
            return;
        }
        pids.push((*task).pid);
    }
}

fn proc_pid_ino(pid: i32) -> u64 {
    0x7000_0000u64 + pid.max(0) as u64
}

#[cfg(test)]
mod tests {
    use alloc::boxed::Box;

    use super::*;
    use crate::fs::kernfs::{KERNFS_DIR_FILE_OPS, inode_for_node, lookup};
    use crate::fs::ops::NOOP_SUPER_OPS;
    use crate::fs::types::{Dentry, File, SuperBlock};
    use crate::kernel::{cred::INIT_CRED, sched, task::TaskStruct};

    static TEST_LOCK: spin::Mutex<()> = spin::Mutex::new(());

    fn with_current<R>(pid: i32, f: impl FnOnce() -> R) -> R {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = pid;
        current.tgid = pid;
        current.cred = &raw const INIT_CRED;
        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);
        }
        let ret = f();
        unsafe {
            sched::set_current(previous);
        }
        ret
    }

    fn proc_root_inode(root: Arc<KernfsNode>) -> InodeRef {
        let sb = SuperBlock::alloc("proc-test", 0x9fa0, &NOOP_SUPER_OPS);
        inode_for_node(&sb, root)
    }

    #[test]
    fn proc_root_lookup_builds_live_pid_directory_dynamically() {
        let _guard = TEST_LOCK.lock();
        with_current(32_001, || {
            let root = new_root();
            populate_root(&root);
            assert!(lookup(&root, "1").is_none());

            let inode = proc_root_inode(root);
            let pid_inode = proc_root_lookup(&inode, "32001").expect("current pid dir");
            assert_eq!(pid_inode.kind, InodeKind::Directory);
            let stat_inode = pid_inode.ops.lookup.unwrap()(&pid_inode, "stat").expect("stat");
            let stat_dentry = Dentry::new_negative("stat");
            stat_dentry.instantiate(stat_inode.clone());
            let stat_file = File::new(stat_dentry, 0, 0, stat_inode.fops);
            let mut buf = [0u8; 128];
            let mut pos = 0;
            let n =
                stat_inode.fops.read.unwrap()(&stat_file, &mut buf, &mut pos).expect("read stat");
            assert!(
                core::str::from_utf8(&buf[..n])
                    .unwrap()
                    .starts_with("32001 ")
            );
            assert!(proc_root_lookup(&inode, "not-a-pid").is_err());
        });
    }

    #[test]
    fn proc_root_readdir_merges_static_entries_and_live_pids() {
        let _guard = TEST_LOCK.lock();
        with_current(32_002, || {
            let root = new_root();
            populate_root(&root);
            let inode = proc_root_inode(root);
            let dentry = Dentry::new_negative("/");
            dentry.instantiate(inode);
            let file = File::new(dentry, 0, 0, &KERNFS_DIR_FILE_OPS);

            let mut names = Vec::new();
            while let Some((name, _, _)) = proc_root_readdir(&file).expect("proc root readdir") {
                names.push(name);
            }

            assert!(names.iter().any(|name| name == "."));
            assert!(names.iter().any(|name| name == ".."));
            assert!(names.iter().any(|name| name == "self"));
            assert!(names.iter().any(|name| name == "version"));
            assert!(names.iter().any(|name| name == "32002"));
        });
    }

    #[test]
    fn proc_root_readdir_only_lists_thread_group_leaders() {
        let mut leader = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        leader.pid = 32_003;
        leader.tgid = 32_003;

        let mut thread = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        thread.pid = 32_004;
        thread.tgid = 32_003;

        let mut pids = Vec::new();
        push_live_pid(&mut pids, &mut *leader);
        push_live_pid(&mut pids, &mut *thread);

        assert_eq!(pids.as_slice(), &[32_003]);
        // Direct lookup of a TID is intentionally still supported by
        // `proc_root_lookup`, matching Linux's invisible-but-addressable
        // `/proc/<tid>` directories.
        assert_eq!(parse_pid_name("32004"), Ok(32_004));
    }
}
