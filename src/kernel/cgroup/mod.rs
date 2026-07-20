//! linux-parity: partial
//! linux-source: vendor/linux/kernel/cgroup
//! cgroup v2 — CPU controller (M32).
//!
//! Mirrors `vendor/linux/kernel/sched/core.c::cpu_cgrp_subsys`.  Lupos M32
//! ships the user-visible cftypes (`cpu.max`, `cpu.weight`, `cpu.weight.nice`,
//! `cpu.stat`, `cpu.idle`, `cpu.pressure`) plus a minimal `TaskGroup` that
//! holds bandwidth state.  The CFS task group plumbing — pulling per-CPU
//! `cfs_rq.runtime_remaining` from a parent group — is structural; full
//! group-scheduling lands in M55 alongside cgroup hierarchy traversal.

pub mod cpu;
pub mod fs;
pub mod namespace;

pub use cpu::{BANDWIDTH_PERIOD_NS_DEFAULT, CpuStat, MAX_BW_BURST, TaskGroup};
pub use fs::{
    CgroupForkState, assign_pid_to_cgroup_fd, assign_pid_to_cgroup_path, finish_pid_cgroup_fork,
    forget_pid_cgroup, mark_pid_exited_from_cgroup, mount, new_cgroup_dir, path_for_pid,
    prepare_pid_cgroup_fork, proc_cgroup_text_for_pid, register, register_cgroup_dir,
    unregister_cgroup_dir,
};
