//! linux-parity: partial
//! linux-source: vendor/linux/init/main.c
//! test-origin: linux:vendor/linux/init/main.c
//! `start_kernel()` tail-order anchors.
//!
//! Linux keeps a dense, named initialization sequence near the tail of
//! `start_kernel()`. Lupos still owns many subsystem implementations outside
//! `init/main.c`, but routing them through these anchors makes the old
//! one-shot calls line up with the source order:
//!
//! `security_init -> net_ns_init -> vfs_caches_init -> signals_init ->
//! cgroup_init -> taskstats_init_early`.

use core::sync::atomic::{AtomicU32, Ordering};

pub const PID_IDR_INIT: u32 = 1 << 0;
pub const ANON_VMA_INIT: u32 = 1 << 1;
pub const THREAD_STACK_CACHE_INIT: u32 = 1 << 2;
pub const CRED_INIT: u32 = 1 << 3;
pub const FORK_INIT: u32 = 1 << 4;
pub const PROC_CACHES_INIT: u32 = 1 << 5;
pub const UTS_NS_INIT: u32 = 1 << 6;
pub const TIME_NS_INIT: u32 = 1 << 7;
pub const KEY_INIT: u32 = 1 << 8;
pub const SECURITY_INIT: u32 = 1 << 9;
pub const DBG_LATE_INIT: u32 = 1 << 10;
pub const NET_NS_INIT: u32 = 1 << 11;
pub const VFS_CACHES_INIT: u32 = 1 << 12;
pub const PAGECACHE_INIT: u32 = 1 << 13;
pub const SIGNALS_INIT: u32 = 1 << 14;
pub const SEQ_FILE_INIT: u32 = 1 << 15;
pub const PROC_ROOT_INIT: u32 = 1 << 16;
pub const NSFS_INIT: u32 = 1 << 17;
pub const PIDFS_INIT: u32 = 1 << 18;
pub const CPUSET_INIT: u32 = 1 << 19;
pub const MEM_CGROUP_INIT: u32 = 1 << 20;
pub const CGROUP_INIT: u32 = 1 << 21;
pub const TASKSTATS_INIT_EARLY: u32 = 1 << 22;
pub const DELAYACCT_INIT: u32 = 1 << 23;
pub const ACPI_SUBSYSTEM_INIT: u32 = 1 << 24;
pub const ARCH_POST_ACPI_SUBSYS_INIT: u32 = 1 << 25;
pub const KCSAN_INIT: u32 = 1 << 26;

static START_KERNEL_TAIL_STATE: AtomicU32 = AtomicU32::new(0);

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct StartKernelTailState {
    bits: u32,
}

impl StartKernelTailState {
    pub const fn contains(self, anchor: u32) -> bool {
        self.bits & anchor != 0
    }

    pub const fn bits(self) -> u32 {
        self.bits
    }
}

fn mark(anchor: u32) {
    START_KERNEL_TAIL_STATE.fetch_or(anchor, Ordering::Release);
}

pub fn state() -> StartKernelTailState {
    StartKernelTailState {
        bits: START_KERNEL_TAIL_STATE.load(Ordering::Acquire),
    }
}

pub fn pid_idr_init() {
    mark(PID_IDR_INIT);
}

pub fn anon_vma_init() {
    mark(ANON_VMA_INIT);
}

pub fn thread_stack_cache_init() {
    mark(THREAD_STACK_CACHE_INIT);
}

pub fn cred_init() {
    mark(CRED_INIT);
}

pub fn fork_init() {
    mark(FORK_INIT);
}

pub fn proc_caches_init() {
    mark(PROC_CACHES_INIT);
}

pub fn uts_ns_init() {
    mark(UTS_NS_INIT);
}

pub fn time_ns_init() {
    mark(TIME_NS_INIT);
}

pub fn key_init() {
    mark(KEY_INIT);
}

pub fn security_init<F: FnOnce()>(init: F) {
    init();
    mark(SECURITY_INIT);
}

pub fn dbg_late_init() {
    mark(DBG_LATE_INIT);
}

pub fn net_ns_init<F: FnOnce()>(init: F) {
    init();
    mark(NET_NS_INIT);
}

pub fn vfs_caches_init<F: FnOnce()>(init: F) {
    init();
    mark(VFS_CACHES_INIT);
}

pub fn pagecache_init() {
    mark(PAGECACHE_INIT);
}

pub fn signals_init() {
    mark(SIGNALS_INIT);
}

pub fn seq_file_init() {
    mark(SEQ_FILE_INIT);
}

pub fn proc_root_init() {
    mark(PROC_ROOT_INIT);
}

pub fn nsfs_init() {
    mark(NSFS_INIT);
}

pub fn pidfs_init() {
    mark(PIDFS_INIT);
}

pub fn cpuset_init() {
    mark(CPUSET_INIT);
}

pub fn mem_cgroup_init() {
    mark(MEM_CGROUP_INIT);
}

pub fn cgroup_init() {
    mark(CGROUP_INIT);
}

pub fn taskstats_init_early<F: FnOnce()>(init: F) {
    init();
    mark(TASKSTATS_INIT_EARLY);
}

pub fn delayacct_init() {
    mark(DELAYACCT_INIT);
}

pub fn acpi_subsystem_init() {
    mark(ACPI_SUBSYSTEM_INIT);
}

pub fn arch_post_acpi_subsys_init() {
    mark(ARCH_POST_ACPI_SUBSYS_INIT);
}

pub fn kcsan_init() {
    mark(KCSAN_INIT);
}

#[cfg(test)]
fn reset_for_test() {
    START_KERNEL_TAIL_STATE.store(0, Ordering::Release);
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use std::vec::Vec;

    #[test]
    fn start_kernel_tail_order_matches_linux_source() {
        let source = std::fs::read_to_string("vendor/linux/init/main.c").unwrap();
        let find = |needle: &str| {
            source
                .find(needle)
                .unwrap_or_else(|| panic!("{needle} missing from Linux init/main.c"))
        };

        assert!(find("security_init();") < find("net_ns_init();"));
        assert!(find("net_ns_init();") < find("vfs_caches_init();"));
        assert!(find("vfs_caches_init();") < find("signals_init();"));
        assert!(find("signals_init();") < find("cgroup_init();"));
        assert!(find("cgroup_init();") < find("taskstats_init_early();"));
        assert!(find("taskstats_init_early();") < find("delayacct_init();"));
        assert!(find("delayacct_init();") < find("acpi_subsystem_init();"));
        assert!(find("acpi_subsystem_init();") < find("arch_post_acpi_subsys_init();"));
        assert!(find("arch_post_acpi_subsys_init();") < find("kcsan_init();"));
    }

    #[test]
    fn wrappers_run_callbacks_and_record_linux_tail_anchors() {
        reset_for_test();
        let mut callbacks = Vec::new();

        pid_idr_init();
        anon_vma_init();
        thread_stack_cache_init();
        cred_init();
        fork_init();
        proc_caches_init();
        uts_ns_init();
        time_ns_init();
        key_init();
        security_init(|| callbacks.push("security"));
        dbg_late_init();
        net_ns_init(|| callbacks.push("net"));
        vfs_caches_init(|| callbacks.push("vfs"));
        pagecache_init();
        signals_init();
        seq_file_init();
        proc_root_init();
        nsfs_init();
        pidfs_init();
        cpuset_init();
        mem_cgroup_init();
        cgroup_init();
        taskstats_init_early(|| callbacks.push("taskstats"));
        delayacct_init();
        acpi_subsystem_init();
        arch_post_acpi_subsys_init();
        kcsan_init();

        assert_eq!(callbacks, ["security", "net", "vfs", "taskstats"]);
        let state = state();
        assert!(state.contains(FORK_INIT));
        assert!(state.contains(PROC_CACHES_INIT));
        assert!(state.contains(SECURITY_INIT));
        assert!(state.contains(NET_NS_INIT));
        assert!(state.contains(VFS_CACHES_INIT));
        assert!(state.contains(SIGNALS_INIT));
        assert!(state.contains(CGROUP_INIT));
        assert!(state.contains(TASKSTATS_INIT_EARLY));
        assert!(state.contains(DELAYACCT_INIT));
        assert!(state.contains(ACPI_SUBSYSTEM_INIT));
        assert!(state.contains(ARCH_POST_ACPI_SUBSYS_INIT));
        assert!(state.contains(KCSAN_INIT));
    }
}
