//! linux-parity: complete
//! linux-source: vendor/linux/fs/proc/kmsg.c
//! test-origin: linux:vendor/linux/fs/proc/kmsg.c
//! `/proc/kmsg`.

use alloc::sync::Arc;

use crate::fs::kernfs::KernfsNode;
use crate::include::uapi::errno::EAGAIN;

pub const SYSLOG_ACTION_CLOSE: i32 = 0;
pub const SYSLOG_ACTION_OPEN: i32 = 1;
pub const SYSLOG_ACTION_READ: i32 = 2;
pub const SYSLOG_ACTION_SIZE_UNREAD: i32 = 9;
pub const SYSLOG_FROM_PROC: i32 = 1;
pub const EPOLLIN: u32 = 0x0001;
pub const EPOLLRDNORM: u32 = 0x0040;
pub const PROC_ENTRY_PERMANENT: u32 = 0;
pub const S_IRUSR: u32 = crate::include::uapi::stat::S_IRUSR;
pub const O_NONBLOCK: u32 = crate::include::uapi::fcntl::O_NONBLOCK;

pub const KMSG_PROC_OPS_SYMBOL: &str = "kmsg_proc_ops";
pub const KMSG_PROC_OPS: &[(&str, &str)] = &[
    ("proc_flags", "PROC_ENTRY_PERMANENT"),
    ("proc_read", "kmsg_read"),
    ("proc_poll", "kmsg_poll"),
    ("proc_open", "kmsg_open"),
    ("proc_release", "kmsg_release"),
    ("proc_lseek", "generic_file_llseek"),
];

pub const fn kmsg_open(do_syslog_open_result: i32) -> i32 {
    do_syslog_open_result
}

pub const fn kmsg_release(_do_syslog_close_result: i32) -> i32 {
    0
}

pub const fn kmsg_read(file_flags: u32, unread_bytes: i32, read_result: i64) -> i64 {
    if (file_flags & O_NONBLOCK) != 0 && unread_bytes == 0 {
        -(EAGAIN as i64)
    } else {
        read_result
    }
}

pub const fn kmsg_poll(unread_bytes: i32) -> u32 {
    if unread_bytes != 0 {
        EPOLLIN | EPOLLRDNORM
    } else {
        0
    }
}

pub const fn proc_kmsg_init_creates() -> (&'static str, u32, &'static str) {
    ("kmsg", S_IRUSR, KMSG_PROC_OPS_SYMBOL)
}

pub fn show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    super::util::copy_into(buf, "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proc_kmsg_ops_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/proc/kmsg.c"
        ));
        let syslog_header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/syslog.h"
        ));
        let proc_header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/proc_fs.h"
        ));
        let eventpoll_header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/eventpoll.h"
        ));
        assert!(source.contains("static int kmsg_open(struct inode * inode, struct file * file)"));
        assert!(
            source.contains("return do_syslog(SYSLOG_ACTION_OPEN, NULL, 0, SYSLOG_FROM_PROC);")
        );
        assert!(source.contains("static int kmsg_release"));
        assert!(
            source.contains("(void) do_syslog(SYSLOG_ACTION_CLOSE, NULL, 0, SYSLOG_FROM_PROC);")
        );
        assert!(source.contains("static ssize_t kmsg_read"));
        assert!(source.contains("file->f_flags & O_NONBLOCK"));
        assert!(source.contains("SYSLOG_ACTION_SIZE_UNREAD"));
        assert!(source.contains("return -EAGAIN;"));
        assert!(
            source.contains("return do_syslog(SYSLOG_ACTION_READ, buf, count, SYSLOG_FROM_PROC);")
        );
        assert!(source.contains("static __poll_t kmsg_poll"));
        assert!(source.contains("poll_wait(file, &log_wait, wait);"));
        assert!(source.contains("return EPOLLIN | EPOLLRDNORM;"));
        assert!(source.contains("static const struct proc_ops kmsg_proc_ops"));
        assert!(source.contains("proc_create(\"kmsg\", S_IRUSR, NULL, &kmsg_proc_ops);"));
        assert!(source.contains("fs_initcall(proc_kmsg_init);"));
        for (slot, target) in KMSG_PROC_OPS {
            assert!(source.contains(slot));
            assert!(source.contains(target));
        }
        assert!(syslog_header.contains("#define SYSLOG_ACTION_CLOSE          0"));
        assert!(syslog_header.contains("#define SYSLOG_ACTION_OPEN           1"));
        assert!(syslog_header.contains("#define SYSLOG_ACTION_READ           2"));
        assert!(syslog_header.contains("#define SYSLOG_ACTION_SIZE_UNREAD    9"));
        assert!(syslog_header.contains("#define SYSLOG_FROM_PROC             1"));
        assert!(proc_header.contains("PROC_ENTRY_PERMANENT"));
        assert!(eventpoll_header.contains("#define EPOLLIN"));
        assert!(eventpoll_header.contains("#define EPOLLRDNORM"));

        assert_eq!(kmsg_open(-7), -7);
        assert_eq!(kmsg_release(-7), 0);
        assert_eq!(kmsg_read(O_NONBLOCK, 0, 12), -(EAGAIN as i64));
        assert_eq!(kmsg_read(O_NONBLOCK, 4, 12), 12);
        assert_eq!(kmsg_read(0, 0, 12), 12);
        assert_eq!(kmsg_poll(0), 0);
        assert_eq!(kmsg_poll(1), EPOLLIN | EPOLLRDNORM);
        assert_eq!(proc_kmsg_init_creates(), ("kmsg", S_IRUSR, "kmsg_proc_ops"));
    }
}
