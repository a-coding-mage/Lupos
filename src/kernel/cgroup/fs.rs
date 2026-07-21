//! linux-parity: partial
//! linux-source: vendor/linux/kernel/cgroup/cgroup.c
//! test-origin: linux:vendor/linux/kernel/cgroup/cgroup.c
//! cgroupfs v2 (M42) — minimal kernfs hierarchy wired to the M32 cpu controller.
//!
//! Mirrors `vendor/linux/kernel/cgroup/cgroup.c`.  M42 ships only the root
//! cgroup with `cpu.max`, `cpu.weight`, `cpu.weight.nice`, `cpu.idle`, and
//! `cpu.stat`.  Hierarchy creation (`mkdir <name>` to spawn a sub-group)
//! lands with the rest of cgroup v2 in M64+.

extern crate alloc;

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::fs::dcache::d_alloc;
use crate::fs::kernfs::{KernfsNode, add_child, inode_for_node, lookup};
use crate::fs::ops::SuperOps;
use crate::fs::super_block::{FileSystemType, register_filesystem};
use crate::fs::types::{SuperBlock, SuperBlockRef};
use crate::include::uapi::errno::{EBADF, EINVAL, ENODEV};
use crate::kernel::cgroup::cpu::{
    TaskGroup, format_cpu_stat, parse_cpu_max, sched_weight_to_cgroup,
};
use crate::kernel::sched::prio::scale_load_down;

const CGROUP2_MAGIC: u64 = 0x63677270;

pub static CGROUP_SUPER_OPS: SuperOps = SuperOps {
    name: "cgroup2",
    statfs: None,
    put_super: None,
    sync_fs: None,
    alloc_inode: None,
    destroy_inode: None,
};

lazy_static! {
    /// Single root cgroup state.  Concrete hierarchy lands in M64+.
    pub static ref ROOT_CG: Mutex<TaskGroup> = Mutex::new(TaskGroup::new_root());
    static ref PID_CGROUPS: Mutex<BTreeMap<i32, PidCgroupMembership>> =
        Mutex::new(BTreeMap::new());
    static ref CGROUP_KILL_SEQUENCES: Mutex<BTreeMap<String, u64>> =
        Mutex::new(BTreeMap::new());
    static ref CGROUP_EVENT_TARGETS: Mutex<BTreeMap<String, CgroupEventTarget>> =
        Mutex::new(BTreeMap::new());
}

#[derive(Clone)]
struct PidCgroupMembership {
    path: String,
    live: bool,
}

struct CgroupEventTarget {
    node: Weak<KernfsNode>,
    sb: Weak<SuperBlock>,
}

/// Snapshot taken before task allocation, matching the cgroup path and
/// `kill_seq` retained in Linux `kernel_clone_args`.
pub struct CgroupForkState {
    path: String,
    kill_seq: u64,
}

// ── cpu.* show/store callbacks ────────────────────────────────────────────

fn cpu_max_show(_n: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let g = ROOT_CG.lock();
    let s = if g.bw_quota == u64::MAX {
        alloc::format!("max {}\n", g.bw_period / 1000)
    } else {
        alloc::format!("{} {}\n", g.bw_quota / 1000, g.bw_period / 1000)
    };
    let n = s.len().min(buf.len());
    buf[..n].copy_from_slice(&s.as_bytes()[..n]);
    Ok(n)
}

fn cpu_max_store(_n: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    let s = core::str::from_utf8(buf).map_err(|_| EINVAL)?;
    let (q, p) = parse_cpu_max(s).ok_or(EINVAL)?;
    let quota_ns = cpu_max_quota_us_to_ns(q)?;
    ROOT_CG.lock().set_max(quota_ns, p).map_err(|_| EINVAL)?;
    Ok(buf.len())
}

fn cpu_max_quota_us_to_ns(quota_us: u64) -> Result<u64, i32> {
    if quota_us == u64::MAX {
        Ok(u64::MAX)
    } else {
        quota_us.checked_mul(1000).ok_or(EINVAL)
    }
}

fn cpu_weight_show(_n: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let g = ROOT_CG.lock();
    let weight = sched_weight_to_cgroup(scale_load_down(g.shares));
    let s = alloc::format!("{}\n", weight);
    let n = s.len().min(buf.len());
    buf[..n].copy_from_slice(&s.as_bytes()[..n]);
    Ok(n)
}

fn cpu_weight_store(_n: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    let s = core::str::from_utf8(buf).map_err(|_| EINVAL)?;
    let w: u64 = s.trim().parse().map_err(|_| EINVAL)?;
    ROOT_CG.lock().set_weight(w).map_err(|_| EINVAL)?;
    Ok(buf.len())
}

fn cpu_weight_nice_store(_n: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    let s = core::str::from_utf8(buf).map_err(|_| EINVAL)?;
    let nice: i32 = s.trim().parse().map_err(|_| EINVAL)?;
    ROOT_CG.lock().set_weight_nice(nice).map_err(|_| EINVAL)?;
    Ok(buf.len())
}

fn cpu_idle_show(_n: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let g = ROOT_CG.lock();
    let s = if g.idle { "1\n" } else { "0\n" };
    let n = s.len().min(buf.len());
    buf[..n].copy_from_slice(&s.as_bytes()[..n]);
    Ok(n)
}

fn cpu_idle_store(_n: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    let s = core::str::from_utf8(buf).map_err(|_| EINVAL)?;
    let v: u32 = s.trim().parse().map_err(|_| EINVAL)?;
    ROOT_CG.lock().set_idle(v != 0);
    Ok(buf.len())
}

fn cpu_stat_show(_n: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let snap = ROOT_CG.lock().stat_snapshot();
    Ok(format_cpu_stat(buf, &snap))
}

fn cgroup_subtree_control_show(_n: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    write_const(buf, "cpu\n")
}

fn cgroup_subtree_control_store(_n: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    let s = core::str::from_utf8(buf).map_err(|_| EINVAL)?;
    for token in s.split_ascii_whitespace() {
        match token {
            "+cpu" | "-cpu" => {}
            _ => return Err(ENODEV),
        }
    }
    Ok(buf.len())
}

fn cgroup_controllers_show(_n: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    write_const(buf, "cpu\n")
}

fn accept_store(_n: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    Ok(buf.len())
}

fn unsupported_controller_store(_n: &Arc<KernfsNode>, _buf: &[u8]) -> Result<usize, i32> {
    Err(ENODEV)
}

fn write_const(buf: &mut [u8], text: &'static str) -> Result<usize, i32> {
    let n = text.len().min(buf.len());
    buf[..n].copy_from_slice(&text.as_bytes()[..n]);
    Ok(n)
}

fn cgroup_type_show(_n: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    write_const(buf, "domain\n")
}

fn cgroup_path_for_node(node: &Arc<KernfsNode>) -> String {
    let dir = node.parent.lock().upgrade().unwrap_or_else(|| node.clone());
    let components = crate::fs::kernfs::symlink::kernfs_node_path_from_root(&dir);
    if components.is_empty() {
        return String::from("/");
    }

    let mut path = String::new();
    for component in components {
        path.push('/');
        path.push_str(&component);
    }
    path
}

fn current_pid() -> Option<i32> {
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        None
    } else {
        Some(unsafe { (*task).pid })
    }
}

pub fn assign_pid_to_cgroup_path(pid: i32, path: &str) {
    let mut memberships = PID_CGROUPS.lock();
    let affected = affected_cgroup_paths(
        memberships
            .get(&pid)
            .map(|membership| membership.path.as_str()),
        Some(path),
    );
    let before = populated_snapshot(&memberships, &affected);
    memberships.insert(
        pid,
        PidCgroupMembership {
            path: String::from(path),
            live: true,
        },
    );
    let changed = populated_transitions(&memberships, &before);
    drop(memberships);
    notify_cgroup_event_transitions(&changed);
}

/// Snapshot a normal fork/clone child's inherited cgroup before task
/// allocation begins.
///
/// Linux `cgroup_css_set_fork()` selects the parent's css_set unless
/// `CLONE_INTO_CGROUP` supplies another target and records that cgroup's
/// `kill_seq` in `kernel_clone_args`.
pub fn prepare_pid_cgroup_fork(parent_pid: i32) -> Option<CgroupForkState> {
    let sequences = CGROUP_KILL_SEQUENCES.lock();
    let memberships = PID_CGROUPS.lock();
    let path = memberships.get(&parent_pid)?.path.clone();
    let kill_seq = sequences.get(&path).copied().unwrap_or(0);
    Some(CgroupForkState { path, kill_seq })
}

/// Link a prepared child before it becomes runnable.
///
/// Returns true when `cgroup.kill` advanced after the pre-fork snapshot, in
/// which case Linux takes the child down immediately after post-fork setup.
pub fn finish_pid_cgroup_fork(state: CgroupForkState, child_pid: i32) -> bool {
    let sequences = CGROUP_KILL_SEQUENCES.lock();
    let mut memberships = PID_CGROUPS.lock();
    let affected = affected_cgroup_paths(None, Some(&state.path));
    let before = populated_snapshot(&memberships, &affected);
    let current_kill_seq = sequences.get(&state.path).copied().unwrap_or(0);
    let crossed_kill = state.kill_seq != current_kill_seq;
    memberships.insert(
        child_pid,
        PidCgroupMembership {
            path: state.path,
            live: true,
        },
    );
    let changed = populated_transitions(&memberships, &before);
    drop(memberships);
    drop(sequences);
    notify_cgroup_event_transitions(&changed);
    crossed_kill
}

pub fn mark_pid_exited_from_cgroup(pid: i32) {
    let mut memberships = PID_CGROUPS.lock();
    let Some(path) = memberships
        .get(&pid)
        .map(|membership| membership.path.clone())
    else {
        return;
    };
    let affected = affected_cgroup_paths(Some(&path), None);
    let before = populated_snapshot(&memberships, &affected);
    if let Some(membership) = memberships.get_mut(&pid) {
        membership.live = false;
    }
    let changed = populated_transitions(&memberships, &before);
    drop(memberships);
    notify_cgroup_event_transitions(&changed);
}

pub fn forget_pid_cgroup(pid: i32) {
    let mut memberships = PID_CGROUPS.lock();
    let Some(path) = memberships
        .get(&pid)
        .map(|membership| membership.path.clone())
    else {
        return;
    };
    let affected = affected_cgroup_paths(Some(&path), None);
    let before = populated_snapshot(&memberships, &affected);
    memberships.remove(&pid);
    let changed = populated_transitions(&memberships, &before);
    drop(memberships);
    notify_cgroup_event_transitions(&changed);
}

fn affected_cgroup_paths(old_path: Option<&str>, new_path: Option<&str>) -> Vec<String> {
    let mut paths = BTreeSet::new();
    for path in [old_path, new_path].into_iter().flatten() {
        let mut cursor = path;
        while cursor != "/" && !cursor.is_empty() {
            paths.insert(String::from(cursor));
            cursor = cursor
                .rsplit_once('/')
                .map(|(parent, _)| if parent.is_empty() { "/" } else { parent })
                .unwrap_or("/");
        }
    }
    paths.into_iter().collect()
}

fn cgroup_path_is_populated_in(
    memberships: &BTreeMap<i32, PidCgroupMembership>,
    path: &str,
) -> bool {
    if path == "/" {
        return true;
    }
    let prefix = alloc::format!("{path}/");
    memberships.values().any(|membership| {
        membership.live && (membership.path == path || membership.path.starts_with(&prefix))
    })
}

fn populated_snapshot(
    memberships: &BTreeMap<i32, PidCgroupMembership>,
    paths: &[String],
) -> Vec<(String, bool)> {
    paths
        .iter()
        .map(|path| (path.clone(), cgroup_path_is_populated_in(memberships, path)))
        .collect()
}

fn populated_transitions(
    memberships: &BTreeMap<i32, PidCgroupMembership>,
    before: &[(String, bool)],
) -> Vec<String> {
    before
        .iter()
        .filter(|(path, was_populated)| {
            *was_populated != cgroup_path_is_populated_in(memberships, path)
        })
        .map(|(path, _)| path.clone())
        .collect()
}

fn notify_cgroup_event_transitions(paths: &[String]) {
    for path in paths {
        let target = CGROUP_EVENT_TARGETS
            .lock()
            .get(path)
            .and_then(|target| Some((target.node.upgrade()?, target.sb.upgrade()?)));
        if let Some((node, sb)) = target {
            // vendor/linux/kernel/cgroup/cgroup.c::cgroup_file_notify() owns
            // the cgroup_file's kernfs_node and notifies it directly. This
            // remains valid after the exiting task detached fs/ns state.
            crate::fs::kernfs::notify_node(&node, &sb);
        }
    }
}

fn cgroup_procs_store(node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    let text = core::str::from_utf8(buf).map_err(|_| EINVAL)?;
    let path = cgroup_path_for_node(node);
    for token in text.split_ascii_whitespace() {
        let mut pid = token.parse::<i32>().map_err(|_| EINVAL)?;
        if pid == 0 {
            pid = current_pid().ok_or(EINVAL)?;
        }
        if pid < 0 {
            return Err(EINVAL);
        }
        assign_pid_to_cgroup_path(pid, &path);
    }
    Ok(buf.len())
}

pub fn cgroup_path_from_fd(fd: i32) -> Result<String, i32> {
    if fd < 0 {
        return Err(EBADF);
    }
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        return Err(EBADF);
    }
    let files = unsafe { crate::kernel::files::get_task_files(task) }.ok_or(EBADF)?;
    let file = files.get(fd)?;
    let path = crate::fs::file::path_hint(&file)
        .or_else(|| crate::fs::mount::path_for_dentry(&file.dentry))
        .unwrap_or_else(|| crate::fs::file::file_path(&file));

    const CGROUP_ROOT: &str = "/sys/fs/cgroup";
    let resolved = if path == CGROUP_ROOT {
        Ok(String::from("/"))
    } else {
        let suffix = path.strip_prefix("/sys/fs/cgroup/").ok_or(EINVAL)?;
        Ok(alloc::format!("/{suffix}"))
    };
    resolved
}

pub fn assign_pid_to_cgroup_fd(pid: i32, fd: i32) -> Result<(), i32> {
    let path = cgroup_path_from_fd(fd)?;
    assign_pid_to_cgroup_path(pid, &path);
    Ok(())
}

fn cgroup_procs_show(node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let path = cgroup_path_for_node(node);
    let mut text = String::new();
    for (pid, membership) in PID_CGROUPS.lock().iter() {
        if membership.live && membership.path == path {
            text.push_str(&alloc::format!("{pid}\n"));
        }
    }
    let n = text.len().min(buf.len());
    buf[..n].copy_from_slice(&text.as_bytes()[..n]);
    Ok(n)
}

pub fn path_for_pid(pid: i32) -> String {
    PID_CGROUPS
        .lock()
        .get(&pid)
        .map(|membership| membership.path.clone())
        .unwrap_or_else(|| String::from("/"))
}

pub fn proc_cgroup_text_for_pid(pid: i32) -> String {
    alloc::format!("0::{}\n", path_for_pid(pid))
}

#[cfg(test)]
fn clear_pid_cgroups_for_test() {
    PID_CGROUPS.lock().clear();
    CGROUP_KILL_SEQUENCES.lock().clear();
    CGROUP_EVENT_TARGETS.lock().clear();
}
fn cgroup_events_show(n: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let path = cgroup_path_for_node(n);
    let populated = if cgroup_path_is_populated(&path) {
        1
    } else {
        0
    };
    let s = alloc::format!("populated {}\nfrozen 0\n", populated);
    let n = s.len().min(buf.len());
    buf[..n].copy_from_slice(&s.as_bytes()[..n]);
    Ok(n)
}

fn cgroup_path_is_populated(path: &str) -> bool {
    cgroup_path_is_populated_in(&PID_CGROUPS.lock(), path)
}
fn cgroup_max_show(_n: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    write_const(buf, "max\n")
}
fn cgroup_zero_show(_n: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    write_const(buf, "0\n")
}
fn cgroup_one_show(_n: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    write_const(buf, "1\n")
}
fn memory_current_show(_n: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    write_const(buf, "0\n")
}
fn memory_events_show(_n: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    write_const(
        buf,
        "low 0\nhigh 0\nmax 0\noom 0\noom_kill 0\noom_group_kill 0\n",
    )
}
fn pids_current_show(_n: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    write_const(buf, "1\n")
}

/// `cgroup.stat` exists on every cgroup, including the root.
/// Ref: `vendor/linux/kernel/cgroup/cgroup.c::cgroup_stat_show` — emits
/// `nr_descendants <N>\nnr_dying_descendants <N>\n` plus a `nr_subsys_<name>`
/// row per controller bound to cgroup v2.  Without active descendants we
/// surface zero for every counter, matching the stable Linux schema.
fn cgroup_stat_show(_n: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let text = "nr_descendants 0\n\
                nr_subsys_cpu 0\n\
                nr_dying_descendants 0\n";
    let n = text.len().min(buf.len());
    buf[..n].copy_from_slice(&text.as_bytes()[..n]);
    Ok(n)
}

/// `cgroup.freeze` is `CFTYPE_NOT_ON_ROOT` — only non-root cgroups expose it.
/// Ref: `vendor/linux/kernel/cgroup/cgroup.c::cgroup_freeze_show`.  Reads
/// surface `"0\n"` for thawed groups; we keep the value at 0 (no freezer
/// state tracked yet) but the file must exist so systemd's
/// `unit_cgroup_freezer_kernel_state` probe in
/// `vendor/systemd/systemd-260.1/src/core/cgroup.c` resolves.
fn cgroup_freeze_show(_n: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    write_const(buf, "0\n")
}

/// `cgroup.freeze` write accepts `0` or `1` and rejects anything else with
/// `-ERANGE` (`EINVAL` is fine for our subset) per `cgroup_freeze_write`.
fn cgroup_freeze_store(_n: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    let s = core::str::from_utf8(buf).map_err(|_| EINVAL)?;
    let trimmed = s.trim();
    match trimmed {
        "0" | "1" => Ok(buf.len()),
        _ => Err(EINVAL),
    }
}

/// `cgroup.kill` is `CFTYPE_NOT_ON_ROOT` and write-only. Writing `1`
/// increments the cgroup kill generation and sends process-directed SIGKILL
/// to every process in this cgroup and its descendants, matching
/// `vendor/linux/kernel/cgroup/cgroup.c::{__cgroup_kill,cgroup_kill}`.
fn cgroup_kill_store(n: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    let s = core::str::from_utf8(buf).map_err(|_| EINVAL)?;
    if s.trim() != "1" {
        return Err(EINVAL);
    }
    let path = cgroup_path_for_node(n);
    let prefix = alloc::format!("{path}/");
    let member_pids = {
        let mut sequences = CGROUP_KILL_SEQUENCES.lock();
        let sequence = sequences.entry(path.clone()).or_insert(0);
        *sequence = sequence.wrapping_add(1);
        PID_CGROUPS
            .lock()
            .iter()
            .filter(|(_, membership)| {
                membership.live && (membership.path == path || membership.path.starts_with(&prefix))
            })
            .map(|(pid, _)| *pid)
            .collect::<BTreeSet<_>>()
    };

    // cgroup.kill is process-directed. Collapse thread memberships to TGIDs
    // before queueing SIGKILL, as Linux's CSS_TASK_ITER_PROCS does.
    let mut target_tgids = BTreeSet::new();
    let mut collect = |task: *mut crate::kernel::task::TaskStruct| {
        if task.is_null() {
            return;
        }
        let pid = unsafe { (*task).pid };
        if member_pids.contains(&pid) {
            target_tgids.insert(unsafe { (*task).tgid });
        }
    };
    crate::kernel::fork::for_each_heap_task(&mut collect);
    crate::kernel::sched::for_each_pool_task(&mut collect);
    for tgid in target_tgids {
        let _ = crate::kernel::signal::send_signal_to_process(tgid, crate::kernel::signal::SIGKILL);
    }
    Ok(buf.len())
}

/// `io.stat` is a read-only nested-keyed file describing per-device I/O
/// counters.  When no block device has accumulated counters the body is
/// empty.  Ref: `vendor/linux/Documentation/admin-guide/cgroup-v2.rst` §
/// "IO Interface Files".  systemd reads this through
/// `vendor/systemd/systemd-260.1/src/core/cgroup.c:3742`.
fn io_stat_show(_n: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    write_const(buf, "")
}

// ── Mount ─────────────────────────────────────────────────────────────────

static SB_INSTANCES: AtomicU64 = AtomicU64::new(0);

pub fn new_cgroup_dir(name: &str, mode: u32) -> Arc<KernfsNode> {
    let dir = KernfsNode::new_dir(name, mode & 0o777);
    populate_cgroup_dir_internal(&dir, /* is_root */ false);
    dir
}

pub fn register_cgroup_dir(dir: &Arc<KernfsNode>, sb: &SuperBlockRef) {
    let components = crate::fs::kernfs::symlink::kernfs_node_path_from_root(dir);
    let path = if components.is_empty() {
        String::from("/")
    } else {
        alloc::format!("/{}", components.join("/"))
    };
    if let Some(events) = lookup(dir, "cgroup.events") {
        CGROUP_EVENT_TARGETS.lock().insert(
            path,
            CgroupEventTarget {
                node: Arc::downgrade(&events),
                sb: Arc::downgrade(sb),
            },
        );
    }
}

pub fn unregister_cgroup_dir(dir: &Arc<KernfsNode>) {
    let components = crate::fs::kernfs::symlink::kernfs_node_path_from_root(dir);
    let path = if components.is_empty() {
        String::from("/")
    } else {
        alloc::format!("/{}", components.join("/"))
    };
    CGROUP_EVENT_TARGETS.lock().remove(&path);
}

fn populate_cgroup_dir(root: &Arc<KernfsNode>) {
    populate_cgroup_dir_internal(root, /* is_root */ true);
}

fn populate_cgroup_dir_internal(root: &Arc<KernfsNode>, is_root: bool) {
    add_child(
        root,
        KernfsNode::new_file(
            "cgroup.controllers",
            0o444,
            Some(cgroup_controllers_show),
            None,
        ),
    );
    add_child(
        root,
        KernfsNode::new_file(
            "cgroup.subtree_control",
            0o644,
            Some(cgroup_subtree_control_show),
            Some(cgroup_subtree_control_store),
        ),
    );
    add_child(
        root,
        KernfsNode::new_file("cpu.max", 0o644, Some(cpu_max_show), Some(cpu_max_store)),
    );
    add_child(
        root,
        KernfsNode::new_file(
            "cpu.weight",
            0o644,
            Some(cpu_weight_show),
            Some(cpu_weight_store),
        ),
    );
    add_child(
        root,
        KernfsNode::new_file("cpu.weight.nice", 0o644, None, Some(cpu_weight_nice_store)),
    );
    add_child(
        root,
        KernfsNode::new_file("cpu.idle", 0o644, Some(cpu_idle_show), Some(cpu_idle_store)),
    );
    add_child(
        root,
        KernfsNode::new_file("cpu.stat", 0o444, Some(cpu_stat_show), None),
    );
    add_child(
        root,
        KernfsNode::new_file("cgroup.type", 0o444, Some(cgroup_type_show), None),
    );
    add_child(
        root,
        KernfsNode::new_file(
            "cgroup.procs",
            0o644,
            Some(cgroup_procs_show),
            Some(cgroup_procs_store),
        ),
    );
    add_child(
        root,
        KernfsNode::new_file(
            "cgroup.threads",
            0o644,
            Some(cgroup_procs_show),
            Some(cgroup_procs_store),
        ),
    );
    add_child(
        root,
        KernfsNode::new_file("cgroup.events", 0o444, Some(cgroup_events_show), None),
    );
    add_child(
        root,
        KernfsNode::new_file(
            "cgroup.max.depth",
            0o644,
            Some(cgroup_max_show),
            Some(accept_store),
        ),
    );
    add_child(
        root,
        KernfsNode::new_file(
            "cgroup.max.descendants",
            0o644,
            Some(cgroup_max_show),
            Some(accept_store),
        ),
    );
    add_child(
        root,
        KernfsNode::new_file("memory.current", 0o444, Some(memory_current_show), None),
    );
    for name in ["memory.min", "memory.low", "memory.high"] {
        add_child(
            root,
            KernfsNode::new_file(
                name,
                0o644,
                Some(cgroup_zero_show),
                Some(unsupported_controller_store),
            ),
        );
    }
    add_child(
        root,
        KernfsNode::new_file(
            "memory.max",
            0o644,
            Some(cgroup_max_show),
            Some(unsupported_controller_store),
        ),
    );
    for name in ["memory.swap.max", "memory.zswap.max"] {
        add_child(
            root,
            KernfsNode::new_file(
                name,
                0o644,
                Some(cgroup_max_show),
                Some(unsupported_controller_store),
            ),
        );
    }
    add_child(
        root,
        KernfsNode::new_file(
            "memory.oom.group",
            0o644,
            Some(cgroup_zero_show),
            Some(unsupported_controller_store),
        ),
    );
    add_child(
        root,
        KernfsNode::new_file(
            "memory.zswap.writeback",
            0o644,
            Some(cgroup_one_show),
            Some(unsupported_controller_store),
        ),
    );
    add_child(
        root,
        KernfsNode::new_file("memory.events", 0o444, Some(memory_events_show), None),
    );
    add_child(
        root,
        KernfsNode::new_file("memory.peak", 0o444, Some(cgroup_zero_show), None),
    );
    add_child(
        root,
        KernfsNode::new_file("memory.swap.peak", 0o444, Some(cgroup_zero_show), None),
    );
    add_child(
        root,
        KernfsNode::new_file("pids.current", 0o444, Some(pids_current_show), None),
    );
    add_child(
        root,
        KernfsNode::new_file(
            "pids.max",
            0o644,
            Some(cgroup_max_show),
            Some(unsupported_controller_store),
        ),
    );
    // cgroup.stat lives on every cgroup; io.stat lives on every cgroup but
    // surfaces an empty body when there is no recorded I/O — matching
    // vendor/linux/Documentation/admin-guide/cgroup-v2.rst.
    add_child(
        root,
        KernfsNode::new_file("cgroup.stat", 0o444, Some(cgroup_stat_show), None),
    );
    add_child(
        root,
        KernfsNode::new_file("io.stat", 0o444, Some(io_stat_show), None),
    );
    // cgroup.freeze and cgroup.kill are `CFTYPE_NOT_ON_ROOT` per
    // `vendor/linux/kernel/cgroup/cgroup.c` (`cgroup_base_files`).  Only
    // populate them on non-root cgroups so the file set matches Linux
    // exactly.  systemd-260.1 probes these through
    // `vendor/systemd/systemd-260.1/src/core/cgroup.c` (freezer state +
    // SIGKILL fan-out).
    if !is_root {
        add_child(
            root,
            KernfsNode::new_file(
                "cgroup.freeze",
                0o644,
                Some(cgroup_freeze_show),
                Some(cgroup_freeze_store),
            ),
        );
        add_child(
            root,
            KernfsNode::new_file("cgroup.kill", 0o200, None, Some(cgroup_kill_store)),
        );
    }
}

pub fn mount(_source: &str, _flags: u64, _data: &str) -> Result<SuperBlockRef, i32> {
    SB_INSTANCES.fetch_add(1, Ordering::AcqRel);
    let sb = SuperBlock::alloc("cgroup2", CGROUP2_MAGIC, &CGROUP_SUPER_OPS);
    let root = KernfsNode::new_dir("/", 0o755);
    populate_cgroup_dir(&root);
    register_cgroup_dir(&root, &sb);

    let root_inode = inode_for_node(&sb, root);
    let root_dentry = d_alloc("/");
    root_dentry.instantiate(root_inode);
    *sb.root.lock() = Some(root_dentry);
    Ok(sb)
}

pub fn register() {
    let _ = register_filesystem(FileSystemType {
        name: "cgroup2",
        mount,
        fs_flags: 0,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_LOCK: spin::Mutex<()> = spin::Mutex::new(());

    #[test]
    fn systemd_root_cgroup_files_have_linux_shaped_defaults() {
        let node = KernfsNode::new_dir("/", 0o755);
        let mut buf = [0u8; 64];

        let n = cgroup_controllers_show(&node, &mut buf).unwrap();
        assert_eq!(&buf[..n], b"cpu\n");

        let n = cgroup_type_show(&node, &mut buf).unwrap();
        assert_eq!(&buf[..n], b"domain\n");

        let root = KernfsNode::new_dir("/", 0o755);
        populate_cgroup_dir(&root);
        let events = lookup(&root, "cgroup.events").unwrap();
        let n = cgroup_events_show(&events, &mut buf).unwrap();
        assert_eq!(&buf[..n], b"populated 1\nfrozen 0\n");

        assert_eq!(cgroup_subtree_control_store(&node, b"+cpu\n"), Ok(5));
        assert_eq!(
            cgroup_subtree_control_store(&node, b"+memory\n"),
            Err(ENODEV)
        );
        assert_eq!(cgroup_subtree_control_store(&node, b"+pids\n"), Err(ENODEV));
    }

    #[test]
    fn cpu_max_store_accepts_max_quota_without_overflow() {
        let node = KernfsNode::new_dir("/", 0o755);

        assert_eq!(cpu_max_store(&node, b"max 100000\n"), Ok(11));
        assert_eq!(cpu_max_quota_us_to_ns(u64::MAX), Ok(u64::MAX));
        assert_eq!(cpu_max_quota_us_to_ns(u64::MAX / 1000 + 1), Err(EINVAL));
    }

    #[test]
    fn cgroup_events_reports_service_cgroups_unpopulated() {
        let _guard = TEST_LOCK.lock();
        clear_pid_cgroups_for_test();
        let root = KernfsNode::new_dir("/", 0o755);
        let service = new_cgroup_dir("systemd-journald.service", 0o755);
        add_child(&root, service.clone());
        let events = lookup(&service, "cgroup.events").unwrap();

        let mut buf = [0u8; 64];
        let n = cgroup_events_show(&events, &mut buf).unwrap();
        assert_eq!(&buf[..n], b"populated 0\nfrozen 0\n");
    }

    #[test]
    fn cgroup_events_reports_descendant_pid_populated() {
        let _guard = TEST_LOCK.lock();
        clear_pid_cgroups_for_test();
        let root = KernfsNode::new_dir("/", 0o755);
        let system = new_cgroup_dir("system.slice", 0o755);
        let service = new_cgroup_dir("systemd-udevd.service", 0o755);
        let udev = new_cgroup_dir("udev", 0o755);
        add_child(&root, system.clone());
        add_child(&system, service.clone());
        add_child(&service, udev.clone());

        assign_pid_to_cgroup_path(342, "/system.slice/systemd-udevd.service/udev");

        let events = lookup(&service, "cgroup.events").unwrap();
        let mut buf = [0u8; 64];
        let n = cgroup_events_show(&events, &mut buf).unwrap();
        assert_eq!(&buf[..n], b"populated 1\nfrozen 0\n");

        let events = lookup(&udev, "cgroup.events").unwrap();
        let n = cgroup_events_show(&events, &mut buf).unwrap();
        assert_eq!(&buf[..n], b"populated 1\nfrozen 0\n");
    }

    #[test]
    fn normal_fork_inherits_membership_until_the_last_task_exits() {
        let _guard = TEST_LOCK.lock();
        clear_pid_cgroups_for_test();
        let service_path = "/user.slice/user-1000.slice/session-c1.scope";

        assign_pid_to_cgroup_path(500, service_path);
        let state = prepare_pid_cgroup_fork(500).expect("parent membership");
        assert!(!finish_pid_cgroup_fork(state, 501));
        assert_eq!(path_for_pid(501), service_path);
        assert!(cgroup_path_is_populated(service_path));

        mark_pid_exited_from_cgroup(500);
        assert!(
            cgroup_path_is_populated(service_path),
            "the live child must keep cgroup.events populated=1"
        );
        mark_pid_exited_from_cgroup(501);
        assert!(
            !cgroup_path_is_populated(service_path),
            "the last exiting task must transition cgroup.events to populated=0"
        );
    }

    #[test]
    fn fork_crossing_cgroup_kill_generation_is_rejected() {
        let _guard = TEST_LOCK.lock();
        clear_pid_cgroups_for_test();
        let service_path = "/user.slice/user-1000.slice/session-c5.scope";

        assign_pid_to_cgroup_path(600, service_path);
        let state = prepare_pid_cgroup_fork(600).expect("pre-kill snapshot");
        CGROUP_KILL_SEQUENCES
            .lock()
            .insert(String::from(service_path), 1);

        assert!(finish_pid_cgroup_fork(state, 601));
        assert_eq!(path_for_pid(601), service_path);
    }

    #[test]
    fn populated_transitions_only_report_visible_value_changes() {
        let _guard = TEST_LOCK.lock();
        clear_pid_cgroups_for_test();
        let service_path = String::from("/system.slice/example.service");
        let paths = alloc::vec![service_path.clone()];

        let before = populated_snapshot(&PID_CGROUPS.lock(), &paths);
        assign_pid_to_cgroup_path(700, &service_path);
        assert_eq!(
            populated_transitions(&PID_CGROUPS.lock(), &before),
            paths,
            "first attachment must notify populated 0 -> 1"
        );

        let before = populated_snapshot(&PID_CGROUPS.lock(), &paths);
        assign_pid_to_cgroup_path(701, &service_path);
        assert!(
            populated_transitions(&PID_CGROUPS.lock(), &before).is_empty(),
            "additional members must not emit a spurious populated transition"
        );

        mark_pid_exited_from_cgroup(700);
        assert!(cgroup_path_is_populated(&service_path));
        let before = populated_snapshot(&PID_CGROUPS.lock(), &paths);
        mark_pid_exited_from_cgroup(701);
        assert_eq!(
            populated_transitions(&PID_CGROUPS.lock(), &before),
            paths,
            "last exit must notify populated 1 -> 0"
        );
    }

    #[test]
    fn cgroup_events_keeps_root_cgroup_populated() {
        let root = KernfsNode::new_dir("/", 0o755);
        populate_cgroup_dir(&root);
        let events = lookup(&root, "cgroup.events").unwrap();

        let mut buf = [0u8; 64];
        let n = cgroup_events_show(&events, &mut buf).unwrap();
        assert_eq!(&buf[..n], b"populated 1\nfrozen 0\n");
    }

    /// Lock in the full set of cgroup v2 control files that systemd 260.1
    /// probes through `vendor/systemd/systemd-260.1/src/basic/cgroup-util.c`
    /// and `.../src/core/cgroup.c`.  CFTYPE_NOT_ON_ROOT files must be
    /// present on non-root cgroups and absent from the root, matching
    /// `vendor/linux/kernel/cgroup/cgroup.c::cgroup_base_files`.
    #[test]
    fn cgroup_root_file_set_matches_systemd_probes() {
        let root = KernfsNode::new_dir("/", 0o755);
        populate_cgroup_dir(&root);

        // Every probe target listed in `cg_get_path(... "cgroup.X")` and
        // `cg_get_path(... "cpu.X" / "memory.X" / "pids.X" / "io.X")` calls
        // through vendor/systemd/systemd-260.1.
        for file in [
            "cgroup.procs",
            "cgroup.threads",
            "cgroup.events",
            "cgroup.type",
            "cgroup.controllers",
            "cgroup.subtree_control",
            "cgroup.max.depth",
            "cgroup.max.descendants",
            "cgroup.stat",
            "cpu.max",
            "cpu.weight",
            "cpu.weight.nice",
            "cpu.idle",
            "cpu.stat",
            "memory.current",
            "memory.events",
            "memory.min",
            "memory.low",
            "memory.high",
            "memory.max",
            "memory.peak",
            "memory.swap.max",
            "memory.swap.peak",
            "memory.zswap.max",
            "memory.zswap.writeback",
            "memory.oom.group",
            "pids.current",
            "pids.max",
            "io.stat",
        ] {
            assert!(
                lookup(&root, file).is_some(),
                "root cgroup missing systemd-probed file: {file}"
            );
        }

        // Root cgroup must NOT expose CFTYPE_NOT_ON_ROOT entries.
        for file in ["cgroup.freeze", "cgroup.kill"] {
            assert!(
                lookup(&root, file).is_none(),
                "root cgroup must not expose CFTYPE_NOT_ON_ROOT file: {file}"
            );
        }

        // Non-root cgroup must expose every NOT_ON_ROOT file.
        let svc = new_cgroup_dir("system-getty.slice", 0o755);
        for file in ["cgroup.freeze", "cgroup.kill", "cgroup.stat"] {
            assert!(
                lookup(&svc, file).is_some(),
                "non-root cgroup missing NOT_ON_ROOT file: {file}"
            );
        }

        // cgroup.stat root output matches Linux's seq_show schema:
        // "nr_descendants 0\nnr_subsys_<...>\nnr_dying_descendants 0\n".
        let stat = lookup(&root, "cgroup.stat").unwrap();
        let mut buf = [0u8; 128];
        let n = cgroup_stat_show(&stat, &mut buf).unwrap();
        let body = core::str::from_utf8(&buf[..n]).unwrap();
        assert!(body.starts_with("nr_descendants 0\n"));
        assert!(body.contains("nr_subsys_cpu 0\n"));
        assert!(body.ends_with("nr_dying_descendants 0\n"));

        // cgroup.freeze read/write contract: read returns "0\n", write
        // accepts "0\n" or "1\n", rejects anything else with EINVAL.
        let freeze = lookup(&svc, "cgroup.freeze").unwrap();
        let n = cgroup_freeze_show(&freeze, &mut buf).unwrap();
        assert_eq!(&buf[..n], b"0\n");
        assert_eq!(cgroup_freeze_store(&freeze, b"1\n"), Ok(2));
        assert_eq!(cgroup_freeze_store(&freeze, b"0\n"), Ok(2));
        assert_eq!(cgroup_freeze_store(&freeze, b"2\n"), Err(EINVAL));

        // cgroup.kill: only accepts "1\n", rejects everything else.
        let kill = lookup(&svc, "cgroup.kill").unwrap();
        assert_eq!(cgroup_kill_store(&kill, b"1\n"), Ok(2));
        assert_eq!(cgroup_kill_store(&kill, b"0\n"), Err(EINVAL));
        assert_eq!(cgroup_kill_store(&kill, b"42\n"), Err(EINVAL));

        // io.stat: empty body on a cgroup with no recorded I/O.
        let io_stat = lookup(&root, "io.stat").unwrap();
        let n = io_stat_show(&io_stat, &mut buf).unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn service_cgroups_expose_systemd_resource_knobs() {
        let system = new_cgroup_dir("system.slice", 0o755);
        let service = new_cgroup_dir("systemd-networkd.service", 0o755);
        add_child(&system, service.clone());

        for file in [
            "cgroup.subtree_control",
            "cgroup.procs",
            "cgroup.threads",
            "memory.min",
            "memory.low",
            "memory.high",
            "memory.max",
            "memory.swap.max",
            "memory.zswap.max",
            "memory.oom.group",
            "memory.zswap.writeback",
            "memory.events",
            "memory.peak",
            "memory.swap.peak",
            "pids.max",
        ] {
            assert!(lookup(&service, file).is_some(), "{file} missing");
        }

        let subtree = lookup(&service, "cgroup.subtree_control").unwrap();
        assert_eq!(cgroup_subtree_control_store(&subtree, b"+cpu\n"), Ok(5));
        assert_eq!(
            cgroup_subtree_control_store(&subtree, b"+cpu +memory +pids\n"),
            Err(ENODEV)
        );
        let memory_high = lookup(&service, "memory.high").unwrap();
        assert_eq!(
            unsupported_controller_store(&memory_high, b"max\n"),
            Err(ENODEV)
        );
        let memory_max = lookup(&service, "memory.max").unwrap();
        assert_eq!(
            unsupported_controller_store(&memory_max, b"1048576\n"),
            Err(ENODEV)
        );
        let pids_max = lookup(&service, "pids.max").unwrap();
        assert_eq!(unsupported_controller_store(&pids_max, b"2\n"), Err(ENODEV));
    }

    #[test]
    fn cgroup_procs_write_updates_proc_pid_cgroup_path() {
        let _guard = TEST_LOCK.lock();
        clear_pid_cgroups_for_test();
        let root = KernfsNode::new_dir("/", 0o755);
        populate_cgroup_dir(&root);
        let system = new_cgroup_dir("system.slice", 0o755);
        let service = new_cgroup_dir("systemd-udevd.service", 0o755);
        add_child(&root, system.clone());
        add_child(&system, service.clone());

        let procs = lookup(&service, "cgroup.procs").unwrap();
        assert_eq!(cgroup_procs_store(&procs, b"341\n"), Ok(4));
        assert_eq!(path_for_pid(341), "/system.slice/systemd-udevd.service");
        assert_eq!(
            proc_cgroup_text_for_pid(341),
            "0::/system.slice/systemd-udevd.service\n"
        );

        let mut buf = [0u8; 32];
        let n = cgroup_procs_show(&procs, &mut buf).unwrap();
        assert_eq!(&buf[..n], b"341\n");
    }

    #[test]
    fn cgroup_exit_drops_procs_but_keeps_proc_cgroup_until_reap() {
        let _guard = TEST_LOCK.lock();
        clear_pid_cgroups_for_test();
        let root = KernfsNode::new_dir("/", 0o755);
        let service = new_cgroup_dir("systemd-udevd.service", 0o755);
        add_child(&root, service.clone());

        assign_pid_to_cgroup_path(342, "/systemd-udevd.service");
        mark_pid_exited_from_cgroup(342);
        assert_eq!(path_for_pid(342), "/systemd-udevd.service");

        let procs = lookup(&service, "cgroup.procs").unwrap();
        let mut buf = [0u8; 64];
        let n = cgroup_procs_show(&procs, &mut buf).unwrap();
        assert_eq!(n, 0);

        let events = lookup(&service, "cgroup.events").unwrap();
        let n = cgroup_events_show(&events, &mut buf).unwrap();
        assert_eq!(&buf[..n], b"populated 0\nfrozen 0\n");

        forget_pid_cgroup(342);
        assert_eq!(path_for_pid(342), "/");
    }
}
