//! linux-parity: complete
//! linux-source: vendor/linux/kernel
//! test-origin: linux:vendor/linux/kernel
//! Lightweight boot-debug trace filters.
//!
//! This is a Lupos-private diagnostic knob, not a Linux UAPI.  Linux already
//! has rich ftrace/dynamic-debug controls; until those are wired end-to-end,
//! `lupos.trace=` keeps ad hoc serial diagnostics opt-in.

use core::sync::atomic::{AtomicI32, AtomicU32, Ordering};

pub const TRACE_SYSCALL: u32 = 1 << 0;
pub const TRACE_FS: u32 = 1 << 1;
pub const TRACE_NETLINK: u32 = 1 << 2;
pub const TRACE_CGROUP: u32 = 1 << 3;
pub const TRACE_PING: u32 = 1 << 4;
pub const TRACE_SYSTEMCTL: u32 = 1 << 5;
pub const TRACE_PROC: u32 = 1 << 6;
pub const TRACE_GLYCIN: u32 = 1 << 7;
pub const TRACE_UDEV: u32 = 1 << 8;
/// Narrow diagnostic for the gdk-pixbuf→glycin bridge: traces only
/// `gdk-pixbuf-*` tool syscalls (failures and readlink*), unlike the very
/// verbose `glycin` flag which also floods the desktop-session comms.
pub const TRACE_PIXBUF: u32 = 1 << 9;
/// Fully-idle stall detector (`kernel/idle_stall.rs`).  Deliberately excluded
/// from `TRACE_ALL`: it must be selected on its own, because every other flag
/// writes to the serial console continuously and that traffic keeps the CPUs
/// awake, which is exactly the condition the detector needs to be absent.
pub const TRACE_STALL: u32 = 1 << 10;
/// Audit CFS intrusive-tree membership at every scheduler mutation.  This is
/// intentionally opt-in: the audit walks the tree and is not part of the
/// normal scheduler fast path.
pub const TRACE_SCHED: u32 = 1 << 11;
/// Trace seccomp decisions and control-plane installation for sandboxed
/// processes. This is opt-in because it runs at the syscall boundary.
pub const TRACE_SECCOMP: u32 = 1 << 12;
/// Trace only Firefox's process-creation, exec, socket, affinity, and
/// sandbox-control syscalls. This is deliberately separate from the broad
/// process trace, whose serial volume changes desktop timing.
pub const TRACE_FIREFOX: u32 = 1 << 13;
pub const TRACE_ALL: u32 = TRACE_SYSCALL
    | TRACE_FS
    | TRACE_NETLINK
    | TRACE_CGROUP
    | TRACE_PING
    | TRACE_SYSTEMCTL
    | TRACE_PROC
    | TRACE_GLYCIN
    | TRACE_PIXBUF
    | TRACE_UDEV;

static TRACE_FLAGS: AtomicU32 = AtomicU32::new(0);
static PING_TRACE_PID: AtomicI32 = AtomicI32::new(-1);

pub fn init_from_cmdline(cmdline: &str) {
    set_flags(parse_cmdline(cmdline));
}

pub fn parse_cmdline(cmdline: &str) -> u32 {
    let mut flags = 0;
    for token in cmdline.split_whitespace() {
        if let Some(value) = token.strip_prefix("lupos.trace=") {
            flags = parse_trace_value(value, flags);
        }
    }
    flags
}

fn parse_trace_value(value: &str, mut flags: u32) -> u32 {
    for item in value.split(',') {
        match item.trim() {
            "" => {}
            "all" => flags |= TRACE_ALL,
            "none" | "off" => flags = 0,
            "syscall" => flags |= TRACE_SYSCALL,
            "fs" | "mount" => flags |= TRACE_FS,
            "netlink" => flags |= TRACE_NETLINK,
            "cgroup" => flags |= TRACE_CGROUP,
            "ping" => flags |= TRACE_PING,
            "systemctl" => flags |= TRACE_SYSTEMCTL,
            "proc" | "process" => flags |= TRACE_PROC,
            "glycin" | "image-loader" => flags |= TRACE_GLYCIN,
            "pixbuf" => flags |= TRACE_PIXBUF,
            "udev" => flags |= TRACE_UDEV,
            "stall" => flags |= TRACE_STALL,
            "sched" | "scheduler" => flags |= TRACE_SCHED,
            "seccomp" | "sandbox" => flags |= TRACE_SECCOMP,
            "firefox" => flags |= TRACE_FIREFOX,
            _ => {}
        }
    }
    flags
}

/// Every flag `set_flags()` will accept.  `TRACE_ALL` is what `lupos.trace=all`
/// selects, which is not the same set: `TRACE_STALL` and `TRACE_SCHED` are
/// valid but opt-in only.
pub const TRACE_KNOWN: u32 = TRACE_ALL | TRACE_STALL | TRACE_SCHED | TRACE_SECCOMP | TRACE_FIREFOX;

pub fn set_flags(flags: u32) {
    let flags = flags & TRACE_KNOWN;
    TRACE_FLAGS.store(flags, Ordering::Release);
    if flags & TRACE_PING == 0 {
        PING_TRACE_PID.store(-1, Ordering::Release);
    }
}

pub fn flags() -> u32 {
    TRACE_FLAGS.load(Ordering::Acquire)
}

pub fn syscall_enabled() -> bool {
    flags() & TRACE_SYSCALL != 0
}

pub fn ping_enabled() -> bool {
    flags() & TRACE_PING != 0
}

pub fn systemctl_enabled() -> bool {
    flags() & TRACE_SYSTEMCTL != 0
}

pub fn proc_enabled() -> bool {
    flags() & TRACE_PROC != 0
}

pub fn glycin_enabled() -> bool {
    flags() & TRACE_GLYCIN != 0
}

pub fn pixbuf_enabled() -> bool {
    flags() & TRACE_PIXBUF != 0
}

pub fn udev_enabled() -> bool {
    flags() & TRACE_UDEV != 0
}

pub fn stall_enabled() -> bool {
    flags() & TRACE_STALL != 0
}

pub fn sched_enabled() -> bool {
    flags() & TRACE_SCHED != 0
}

pub fn seccomp_enabled() -> bool {
    flags() & TRACE_SECCOMP != 0
}

pub fn firefox_enabled() -> bool {
    flags() & TRACE_FIREFOX != 0
}

pub fn remember_ping_pid_for_exec(pid: i32, path: &str, exec_path: &str) -> bool {
    if !ping_enabled() {
        return false;
    }
    if basename_is(path, "ping") || basename_is(exec_path, "ping") {
        PING_TRACE_PID.store(pid, Ordering::Release);
        true
    } else {
        if PING_TRACE_PID.load(Ordering::Acquire) == pid {
            PING_TRACE_PID.store(-1, Ordering::Release);
        }
        false
    }
}

pub fn ping_pid_matches(pid: i32) -> bool {
    ping_enabled() && PING_TRACE_PID.load(Ordering::Acquire) == pid
}

fn basename_is(path: &str, name: &str) -> bool {
    path.rsplit('/').next().unwrap_or(path) == name
}

pub fn fs_enabled() -> bool {
    flags() & TRACE_FS != 0
}

#[allow(dead_code)]
pub fn netlink_enabled() -> bool {
    flags() & TRACE_NETLINK != 0
}

#[allow(dead_code)]
pub fn cgroup_enabled() -> bool {
    flags() & TRACE_CGROUP != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_LOCK: spin::Mutex<()> = spin::Mutex::new(());

    fn reset_trace_state() -> spin::MutexGuard<'static, ()> {
        let guard = TEST_LOCK.lock();
        set_flags(0);
        PING_TRACE_PID.store(-1, Ordering::Release);
        guard
    }

    #[test]
    fn parses_lupos_trace_cmdline_as_comma_list() {
        let _guard = reset_trace_state();
        let flags = parse_cmdline(
            "quiet lupos.trace=syscall,fs,cgroup,ping,systemctl,proc,glycin,pixbuf,udev,sched,seccomp,firefox root=/dev/vda1",
        );

        assert_ne!(flags & TRACE_SYSCALL, 0);
        assert_ne!(flags & TRACE_FS, 0);
        assert_eq!(flags & TRACE_NETLINK, 0);
        assert_ne!(flags & TRACE_CGROUP, 0);
        assert_ne!(flags & TRACE_PING, 0);
        assert_ne!(flags & TRACE_SYSTEMCTL, 0);
        assert_ne!(flags & TRACE_PROC, 0);
        assert_ne!(flags & TRACE_GLYCIN, 0);
        assert_ne!(flags & TRACE_PIXBUF, 0);
        assert_ne!(flags & TRACE_UDEV, 0);
        assert_ne!(flags & TRACE_SCHED, 0);
        assert_ne!(flags & TRACE_SECCOMP, 0);
        assert_ne!(flags & TRACE_FIREFOX, 0);
    }

    #[test]
    fn parse_supports_all_and_later_off() {
        let _guard = reset_trace_state();
        assert_eq!(parse_cmdline("lupos.trace=all"), TRACE_ALL);
        assert_eq!(parse_cmdline("lupos.trace=all lupos.trace=off"), 0);
    }

    #[test]
    fn global_flags_are_masked() {
        let _guard = reset_trace_state();
        set_flags(TRACE_SYSCALL | (1 << 31));

        assert!(syscall_enabled());
        assert_eq!(flags() & (1 << 31), 0);

        set_flags(0);
    }

    #[test]
    fn scheduler_trace_is_opt_in_and_not_selected_by_all() {
        let _guard = reset_trace_state();
        assert_eq!(parse_cmdline("lupos.trace=all"), TRACE_ALL);
        assert_eq!(parse_cmdline("lupos.trace=sched"), TRACE_SCHED);
        set_flags(TRACE_SCHED);
        assert!(sched_enabled());
        set_flags(0);
        assert!(!sched_enabled());
    }

    #[test]
    fn seccomp_trace_is_opt_in_and_not_selected_by_all() {
        let _guard = reset_trace_state();
        assert_eq!(parse_cmdline("lupos.trace=all"), TRACE_ALL);
        assert_eq!(parse_cmdline("lupos.trace=seccomp"), TRACE_SECCOMP);
        set_flags(TRACE_SECCOMP);
        assert!(seccomp_enabled());
        set_flags(0);
        assert!(!seccomp_enabled());
    }

    #[test]
    fn ping_trace_remembers_exec_pid_by_basename() {
        let _guard = reset_trace_state();
        set_flags(TRACE_PING);

        assert!(remember_ping_pid_for_exec(
            326,
            "/usr/bin/ping",
            "/usr/bin/ping"
        ));
        assert!(ping_pid_matches(326));
        assert!(!ping_pid_matches(325));

        assert!(!remember_ping_pid_for_exec(326, "/bin/bash", "/bin/bash"));
        assert!(!ping_pid_matches(326));

        set_flags(0);
    }
}
