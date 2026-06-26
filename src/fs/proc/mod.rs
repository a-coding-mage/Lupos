//! linux-parity: partial
//! linux-source: vendor/linux/fs/proc
//! test-origin: linux:vendor/linux/fs/proc
//! procfs kernel-side `/proc` skeleton on kernfs.

extern crate alloc;

use core::sync::atomic::Ordering;

use crate::fs::dcache::d_alloc;
use crate::fs::kernfs::inode_for_node;
use crate::fs::ops::SuperOps;
use crate::fs::super_block::{FileSystemType, register_filesystem};
use crate::fs::types::{SuperBlock, SuperBlockRef};

pub mod array;
pub mod base;
pub mod bootconfig;
pub mod cmdline;
pub mod consoles;
pub mod cpuinfo;
pub mod devices;
pub mod fd;
pub mod generic;
pub mod inode;
pub mod interrupts;
pub mod kcore;
pub mod kmsg;
pub mod loadavg;
pub mod meminfo;
pub mod namespaces;
pub mod nommu;
pub mod page;
pub mod proc_net;
pub mod proc_sysctl;
pub mod proc_tty;
pub mod root;
#[path = "self.rs"]
pub mod self_;
pub mod softirqs;
pub mod stat;
pub mod swaps;
pub mod task_mmu;
pub mod task_nommu;
pub mod thread_self;
pub mod uptime;
pub mod util;
pub mod version;
pub mod vmcore;

const PROC_SUPER_MAGIC: u64 = 0x9fa0;

pub static PROCFS_SUPER_OPS: SuperOps = SuperOps {
    name: "proc",
    statfs: None,
    put_super: None,
    sync_fs: None,
    alloc_inode: None,
    destroy_inode: None,
};

pub fn mount(_source: &str, _flags: u64, _data: &str) -> Result<SuperBlockRef, i32> {
    let sb = SuperBlock::alloc("proc", PROC_SUPER_MAGIC, &PROCFS_SUPER_OPS);
    let root = root::new_root();
    root::populate_root(&root);

    let root_inode = inode_for_node(&sb, root);
    let root_dentry = d_alloc("/");
    root_dentry.instantiate(root_inode);
    *sb.root.lock() = Some(root_dentry);
    let _ = sb.next_ino.fetch_add(0, Ordering::Relaxed);
    Ok(sb)
}

pub fn register() {
    let _ = register_filesystem(FileSystemType {
        name: "proc",
        mount,
        fs_flags: 0,
    });
}

#[cfg(test)]
mod tests {
    use crate::fs::file::alloc_file;
    use crate::fs::read_write::vfs_read;
    use crate::include::uapi::fcntl::O_RDONLY;

    #[test]
    fn procfs_mount_exposes_global_and_self_schema() {
        crate::fs::init();
        let sb = super::mount("", 0, "").unwrap();
        let root = sb.root().unwrap();
        let root_inode = root.inode().unwrap();
        let lookup = root_inode.ops.lookup.unwrap();
        assert!(lookup(&root_inode, "version").is_ok());
        assert!(lookup(&root_inode, "self").is_ok());

        let meminfo_inode = lookup(&root_inode, "meminfo").unwrap();
        let dentry = crate::fs::dcache::d_alloc("meminfo");
        dentry.instantiate(meminfo_inode.clone());
        let file = alloc_file(dentry, O_RDONLY, 0, meminfo_inode.fops);
        let mut buf = [0u8; 128];
        let n = vfs_read(&file, &mut buf).unwrap();
        let text = core::str::from_utf8(&buf[..n]).unwrap();
        assert!(text.contains("MemTotal:"));
    }

    /// Lock the /proc surface that systemd-260.1 probes during early boot.
    /// Sources for the probe set:
    /// - `vendor/systemd/systemd-260.1/src/basic/proc-cmdline.c`
    /// - `vendor/systemd/systemd-260.1/src/basic/virt.c`
    /// - `vendor/systemd/systemd-260.1/src/core/manager.c`
    /// - `vendor/systemd/systemd-260.1/src/login/logind.c`
    /// Each of these reads at least one of the files below, and a missing
    /// entry surfaces as an `ENOENT` boot-time error in the journal.
    #[test]
    fn procfs_root_exposes_systemd_probe_surface() {
        crate::fs::init();
        let sb = super::mount("", 0, "").unwrap();
        let root = sb.root().unwrap();
        let root_inode = root.inode().unwrap();
        let lookup = root_inode.ops.lookup.unwrap();

        for name in [
            "version",
            "uptime",
            "loadavg",
            "meminfo",
            "stat",
            "cpuinfo",
            "cmdline",
            "filesystems",
            "mounts",
            "mountinfo",
            "swaps",
            "devices",
            "interrupts",
            "softirqs",
            "consoles",
            "kmsg",
            "lupos_boot_trace",
            "self",
            "thread-self",
            "net",
            "sys",
            "tty",
        ] {
            assert!(
                lookup(&root_inode, name).is_ok(),
                "/proc/{name} must resolve for systemd-260.1 probes"
            );
        }

        // /proc/sys/kernel/random/boot_id is what systemd uses to seed the
        // boot session identifier (`man 5 machine-info`).  Ref:
        // vendor/linux/drivers/char/random.c::random_table.
        let sys_inode = lookup(&root_inode, "sys").unwrap();
        let sys_lookup = sys_inode.ops.lookup.unwrap();
        let kernel_inode = sys_lookup(&sys_inode, "kernel").expect("/proc/sys/kernel");
        let kernel_lookup = kernel_inode.ops.lookup.unwrap();
        for entry in ["hostname", "osrelease", "ostype", "version", "random"] {
            assert!(
                kernel_lookup(&kernel_inode, entry).is_ok(),
                "/proc/sys/kernel/{entry} must exist (systemd reads this on boot)"
            );
        }
        let random_inode = kernel_lookup(&kernel_inode, "random").unwrap();
        let random_lookup = random_inode.ops.lookup.unwrap();
        assert!(
            random_lookup(&random_inode, "boot_id").is_ok(),
            "/proc/sys/kernel/random/boot_id must exist for systemd's boot session"
        );

        // /proc/sys/fs/{file-max,nr_open} are read by systemd's
        // exec_context to honour `LimitNOFILE=infinity` per
        // vendor/systemd/systemd-260.1/src/core/execute.c.
        let fs_inode = sys_lookup(&sys_inode, "fs").expect("/proc/sys/fs");
        let fs_lookup = fs_inode.ops.lookup.unwrap();
        for entry in ["file-max", "nr_open", "mqueue"] {
            assert!(
                fs_lookup(&fs_inode, entry).is_ok(),
                "/proc/sys/fs/{entry} must exist"
            );
        }
        let mqueue_inode = fs_lookup(&fs_inode, "mqueue").expect("/proc/sys/fs/mqueue");
        let mqueue_lookup = mqueue_inode.ops.lookup.unwrap();
        for entry in ["queues_max", "msg_max", "msgsize_max"] {
            assert!(
                mqueue_lookup(&mqueue_inode, entry).is_ok(),
                "/proc/sys/fs/mqueue/{entry} must exist"
            );
        }
    }
}
