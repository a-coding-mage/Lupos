//! linux-parity: partial
//! linux-source: vendor/linux/fs/proc/root.c
//! procfs root directory population.
//!
//! Ref: `vendor/linux/fs/proc/root.c`

use alloc::sync::Arc;

use crate::fs::kernfs::{KernfsNode, add_child};

pub fn new_root() -> Arc<KernfsNode> {
    KernfsNode::new_dir("/", 0o555)
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
    let pid1 = KernfsNode::new_dir("1", 0o555);
    super::base::add_task_common(&pid1);
    add_child(root, pid1);
    add_child(root, super::thread_self::new_thread_self());
    add_child(root, super::proc_net::new_net_dir());
    add_child(root, super::proc_sysctl::new_sys_dir());
    add_child(root, super::proc_tty::new_tty_dir());
}
