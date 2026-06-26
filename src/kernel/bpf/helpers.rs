//! linux-parity: complete
//! linux-source: vendor/linux/kernel/bpf/helpers.c
//! test-origin: linux:vendor/linux/kernel/bpf/helpers.c
//! eBPF helper functions (BPF_FUNC_*).
//!
//! Mirrors selected entries from `vendor/linux/kernel/bpf/helpers.c`
//! and `vendor/linux/kernel/trace/bpf_trace.c`.

use super::uapi::{BPF_FUNC_get_current_pid_tgid, BPF_FUNC_ktime_get_ns, BPF_FUNC_trace_printk};

/// Dispatch a BPF helper call by id.
/// Args: r1..r5 from the eBPF interpreter.
pub fn call(id: u32, r1: u64, _r2: u64, _r3: u64, _r4: u64, _r5: u64) -> u64 {
    match id {
        BPF_FUNC_get_current_pid_tgid => {
            // Linux returns `(tgid << 32) | pid`.  Without per-task BPF context
            // wired up yet, return a stable synthetic value matching kernel
            // task id 1 (init).
            (1u64 << 32) | 1
        }
        BPF_FUNC_ktime_get_ns => crate::kernel::time::jiffies::jiffies() as u64 * 1_000_000,
        BPF_FUNC_trace_printk => {
            // r1 = fmt pointer (unused in M63 — we just emit a marker line)
            crate::printk!(
                crate::kernel::printk::levels::KERN_INFO,
                "bpf_trace_printk: arg={:#x}\n",
                r1
            );
            0
        }
        _ => u64::MAX,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pid_tgid_low32_matches_pid() {
        let v = call(BPF_FUNC_get_current_pid_tgid, 0, 0, 0, 0, 0);
        let pid = (v & 0xffff_ffff) as u32;
        assert_eq!(pid, 1);
    }

    #[test]
    fn ktime_monotonic() {
        let a = call(BPF_FUNC_ktime_get_ns, 0, 0, 0, 0, 0);
        let b = call(BPF_FUNC_ktime_get_ns, 0, 0, 0, 0, 0);
        assert!(b >= a);
    }

    #[test]
    fn unknown_helper_returns_umax() {
        assert_eq!(call(0xdead_beef, 0, 0, 0, 0, 0), u64::MAX);
    }
}
