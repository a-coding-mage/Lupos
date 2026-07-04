//! linux-parity: partial
//! linux-source: vendor/linux/init/init_task.c
//! test-origin: linux:vendor/linux/init/init_task.c
//! Initial task template.
//!
//! Models the fields from `vendor/linux/init/init_task.c` that Lupos
//! represents today: the boot task is a kernel thread, starts runnable,
//! owns root credentials, uses normal scheduling at nice 0, and begins with
//! a single-thread signal group.
//! Linux also defines init_signals, init_sighand, init_cred, init_groups,
//! init_task_exec_state, pid links, namespace/audit hooks, and optional
//! shadow-call-stack storage; those globals remain owned by their subsystem
//! modules or deferred.

use crate::kernel::sched::entity::CpuMask;
use crate::kernel::sched::prio::{DEFAULT_PRIO, MAX_PRIO, SCHED_NORMAL};
use crate::kernel::task::{TASK_COMM_LEN, task_state};

pub const PF_KTHREAD: u32 = 0x0020_0000;
pub const INIT_TASK_COMM: &str = "swapper";
pub const INIT_TASK_USAGE: usize = 2;
pub const INIT_SIGNAL_THREADS: u32 = 1;
pub const INIT_TIMER_SLACK_NS: u64 = 50_000;

#[derive(Clone, Copy, Debug)]
pub struct InitTaskTemplate {
    pub state: u32,
    pub usage: usize,
    pub flags: u32,
    pub prio: i32,
    pub static_prio: i32,
    pub normal_prio: i32,
    pub policy: u32,
    pub cpus_mask: CpuMask,
    pub nr_cpus_allowed: i32,
    pub mm_is_null: bool,
    pub signal_threads: u32,
    pub comm: [u8; TASK_COMM_LEN],
    pub timer_slack_ns: u64,
}

pub const fn init_task_template() -> InitTaskTemplate {
    InitTaskTemplate {
        state: task_state::TASK_RUNNING,
        usage: INIT_TASK_USAGE,
        flags: PF_KTHREAD,
        prio: MAX_PRIO - 20,
        static_prio: MAX_PRIO - 20,
        normal_prio: MAX_PRIO - 20,
        policy: SCHED_NORMAL,
        cpus_mask: CpuMask::all(),
        nr_cpus_allowed: 64,
        mm_is_null: true,
        signal_threads: INIT_SIGNAL_THREADS,
        comm: pack_comm(INIT_TASK_COMM),
        timer_slack_ns: INIT_TIMER_SLACK_NS,
    }
}

pub const fn default_sched_prio() -> i32 {
    DEFAULT_PRIO
}

const fn pack_comm(name: &str) -> [u8; TASK_COMM_LEN] {
    let bytes = name.as_bytes();
    let mut out = [0u8; TASK_COMM_LEN];
    let limit = if bytes.len() < TASK_COMM_LEN - 1 {
        bytes.len()
    } else {
        TASK_COMM_LEN - 1
    };
    let mut i = 0;
    while i < limit {
        out[i] = bytes[i];
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_task_template_matches_linux_boot_defaults() {
        let template = init_task_template();
        assert_eq!(template.state, task_state::TASK_RUNNING);
        assert_eq!(template.usage, 2);
        assert_eq!(template.flags & PF_KTHREAD, PF_KTHREAD);
        assert_eq!(template.prio, MAX_PRIO - 20);
        assert_eq!(template.static_prio, MAX_PRIO - 20);
        assert_eq!(template.normal_prio, MAX_PRIO - 20);
        assert_eq!(template.policy, SCHED_NORMAL);
        assert_eq!(template.cpus_mask.0, CpuMask::all().0);
        assert_eq!(template.signal_threads, 1);
        assert_eq!(&template.comm[..7], b"swapper");
        assert_eq!(template.timer_slack_ns, 50_000);
    }
}
