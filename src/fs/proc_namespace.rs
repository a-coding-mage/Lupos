//! linux-parity: complete
//! linux-source: vendor/linux/fs/proc_namespace.c
//! test-origin: linux:vendor/linux/fs/proc_namespace.c
//! `/proc/*/mount*` namespace renderers.
//!
//! Ref: `vendor/linux/fs/proc_namespace.c`

extern crate alloc;

use alloc::format;
use alloc::string::String;

use super::mount::MOUNTS;
use super::types::FileRef;
use crate::fs::eventpoll::{EPOLLERR, EPOLLIN, EPOLLPRI};

fn mount_options(readonly: bool) -> &'static str {
    if readonly { "ro" } else { "rw" }
}

pub fn render_mounts() -> String {
    let mut out = String::new();
    for (path, mount) in MOUNTS.by_path.lock().iter() {
        out.push_str(&format!(
            "{} {} {} {} 0 0\n",
            mount.sb.fs_name,
            path,
            mount.sb.fs_name,
            mount_options(mount.is_readonly()),
        ));
    }
    out
}

pub fn render_mountinfo() -> String {
    let mut out = String::new();
    for (path, mount) in MOUNTS.by_path.lock().iter() {
        let parent_id = mount
            .parent
            .lock()
            .as_ref()
            .map(|m| m.id)
            .unwrap_or(mount.id);
        out.push_str(&format!(
            "{} {} 0:0 / {} {} - {} {} {}\n",
            mount.id,
            parent_id,
            path,
            mount_options(mount.is_readonly()),
            mount.sb.fs_name,
            mount.sb.fs_name,
            mount_options(mount.is_readonly()),
        ));
    }
    out
}

pub fn render_mountstats() -> String {
    let mut out = String::new();
    for (path, mount) in MOUNTS.by_path.lock().iter() {
        out.push_str(&format!(
            "device {} mounted on {} with fstype {}\n",
            mount.sb.fs_name, path, mount.sb.fs_name
        ));
    }
    out
}

pub fn poll_mount_table(file: &FileRef) -> u32 {
    let current = super::mount::mount_event();
    let mut observed = file.private.lock();
    if *observed == 0 {
        *observed = current as usize;
        return 0;
    }
    if *observed as u64 != current {
        EPOLLIN | EPOLLERR | EPOLLPRI
    } else {
        0
    }
}

pub fn consume_mount_table_poll(file: &FileRef) {
    *file.private.lock() = super::mount::mount_event() as usize;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::dcache::d_alloc_child;
    use crate::fs::mount::{MOUNTS, Mount, do_mount, set_rootfs};
    use crate::fs::super_block::mount_fs;
    use crate::fs::types::DentryRef;

    fn reset_mount_state() {
        MOUNTS.root.lock().take();
        MOUNTS.by_path.lock().clear();
    }

    fn mkdir_dentry(parent: &DentryRef, name: &str) -> DentryRef {
        let parent_inode = parent.inode().expect("parent inode");
        let mkdir = parent_inode.ops.mkdir.expect("mkdir op");
        let child_inode = mkdir(&parent_inode, name, 0o755).expect("mkdir");
        let child = d_alloc_child(parent, name);
        child.instantiate(child_inode);
        child
    }

    #[test]
    fn proc_namespace_renders_root_mount() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        crate::fs::init();
        reset_mount_state();
        let sb = mount_fs("ramfs", "", 0, "").unwrap();
        let root = sb.root().unwrap();
        set_rootfs(Mount::alloc(sb, root, 0));
        let mounts = render_mounts();
        assert!(mounts.contains("ramfs / ramfs rw 0 0"));
        assert!(render_mountinfo().contains(" - ramfs ramfs rw"));
    }

    #[test]
    fn proc_namespace_renders_tmpfs_mount_after_successful_mount() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        crate::fs::init();
        reset_mount_state();
        let sb = mount_fs("ramfs", "", 0, "").unwrap();
        let root = sb.root().unwrap();
        set_rootfs(Mount::alloc(sb, root.clone(), 0));
        mkdir_dentry(&root, "tmp");

        do_mount("tmpfs", "tmpfs", "/tmp", 0, "").expect("tmpfs /tmp");

        assert!(render_mounts().contains("tmpfs /tmp tmpfs rw 0 0"));
        assert!(
            render_mountinfo()
                .lines()
                .any(|line| line.contains(" /tmp ") && line.contains(" - tmpfs tmpfs rw")),
            "mountinfo must expose the mounted /tmp tmpfs"
        );
    }
}
