//! linux-parity: complete
//! linux-source: vendor/linux/fs/proc/stat.c
//! test-origin: linux:vendor/linux/fs/proc/stat.c
//! `/proc/stat`.
//!
//! Ref: `vendor/linux/fs/proc/stat.c`

use alloc::format;
use alloc::string::String;
use alloc::sync::Arc;

use crate::fs::kernfs::KernfsNode;
use crate::kernel::softirq::NR_SOFTIRQS;

pub const NSEC_PER_USEC: u64 = 1_000;
pub const NSEC_PER_SEC: u64 = 1_000_000_000;
pub const USER_HZ: u64 = 100;

pub const CPUTIME_USER: usize = 0;
pub const CPUTIME_NICE: usize = 1;
pub const CPUTIME_SYSTEM: usize = 2;
pub const CPUTIME_SOFTIRQ: usize = 3;
pub const CPUTIME_IRQ: usize = 4;
pub const CPUTIME_IDLE: usize = 5;
pub const CPUTIME_IOWAIT: usize = 6;
pub const CPUTIME_STEAL: usize = 7;
pub const CPUTIME_GUEST: usize = 8;
pub const CPUTIME_GUEST_NICE: usize = 9;
pub const NR_STATS: usize = 10;

pub const PROC_ENTRY_PERMANENT: u32 = 0;
pub const STAT_PROC_OPS_SYMBOL: &str = "stat_proc_ops";
pub const STAT_PROC_OPS: &[(&str, &str)] = &[
    ("proc_flags", "PROC_ENTRY_PERMANENT"),
    ("proc_open", "stat_open"),
    ("proc_read_iter", "seq_read_iter"),
    ("proc_lseek", "seq_lseek"),
    ("proc_release", "single_release"),
];

pub const PROC_STAT_OPEN_BASE_SIZE: usize = 1024;
pub const PROC_STAT_OPEN_BYTES_PER_CPU: usize = 128;
pub const PROC_STAT_OPEN_BYTES_PER_IRQ: usize = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProcStatCpu {
    pub online: bool,
    pub cpustat: [u64; NR_STATS],
    pub irq_sum: u64,
    pub arch_irq_stat: u64,
    pub softirqs: [u64; NR_SOFTIRQS],
    pub idle_time_us: Option<u64>,
    pub iowait_time_us: Option<u64>,
}

impl ProcStatCpu {
    pub const fn zero(online: bool) -> Self {
        Self {
            online,
            cpustat: [0; NR_STATS],
            irq_sum: 0,
            arch_irq_stat: 0,
            softirqs: [0; NR_SOFTIRQS],
            idle_time_us: None,
            iowait_time_us: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProcStatSnapshot<'a> {
    pub possible_cpus: &'a [ProcStatCpu],
    pub irq_counts: &'a [u64],
    pub arch_irq_stat: u64,
    pub context_switches: u64,
    pub boot_time_sec: u64,
    pub total_forks: u64,
    pub procs_running: u32,
    pub procs_blocked: u32,
}

const DEFAULT_CPU: ProcStatCpu = ProcStatCpu::zero(true);
const DEFAULT_CPUS: [ProcStatCpu; 1] = [DEFAULT_CPU];
const DEFAULT_IRQ_COUNTS: [u64; 0] = [];

pub const DEFAULT_PROC_STAT_SNAPSHOT: ProcStatSnapshot<'static> = ProcStatSnapshot {
    possible_cpus: &DEFAULT_CPUS,
    irq_counts: &DEFAULT_IRQ_COUNTS,
    arch_irq_stat: 0,
    context_switches: 0,
    boot_time_sec: 0,
    total_forks: 1,
    procs_running: 0,
    procs_blocked: 0,
};

pub const fn nsec_to_clock_t(nsec: u64) -> u64 {
    nsec / (NSEC_PER_SEC / USER_HZ)
}

pub const fn get_idle_time(cpu: &ProcStatCpu) -> u64 {
    if cpu.online {
        if let Some(idle_time_us) = cpu.idle_time_us {
            return idle_time_us * NSEC_PER_USEC;
        }
    }
    cpu.cpustat[CPUTIME_IDLE]
}

pub const fn get_iowait_time(cpu: &ProcStatCpu) -> u64 {
    if cpu.online {
        if let Some(iowait_time_us) = cpu.iowait_time_us {
            return iowait_time_us * NSEC_PER_USEC;
        }
    }
    cpu.cpustat[CPUTIME_IOWAIT]
}

pub const fn stat_open_size(num_online_cpus: usize, nr_irqs: usize) -> usize {
    PROC_STAT_OPEN_BASE_SIZE
        + PROC_STAT_OPEN_BYTES_PER_CPU * num_online_cpus
        + PROC_STAT_OPEN_BYTES_PER_IRQ * nr_irqs
}

fn push_cpu_times(out: &mut String, prefix: &str, times: &[u64; 10]) {
    out.push_str(prefix);
    out.push_str(&format!("{}", nsec_to_clock_t(times[0])));
    for value in &times[1..] {
        out.push_str(&format!(" {}", nsec_to_clock_t(*value)));
    }
    out.push('\n');
}

fn cpu_times(cpu: &ProcStatCpu) -> [u64; 10] {
    [
        cpu.cpustat[CPUTIME_USER],
        cpu.cpustat[CPUTIME_NICE],
        cpu.cpustat[CPUTIME_SYSTEM],
        get_idle_time(cpu),
        get_iowait_time(cpu),
        cpu.cpustat[CPUTIME_IRQ],
        cpu.cpustat[CPUTIME_SOFTIRQ],
        cpu.cpustat[CPUTIME_STEAL],
        cpu.cpustat[CPUTIME_GUEST],
        cpu.cpustat[CPUTIME_GUEST_NICE],
    ]
}

pub fn render_proc_stat(snapshot: &ProcStatSnapshot<'_>) -> String {
    let mut out = String::new();
    let mut total = [0u64; 10];
    let mut irq_sum = 0u64;
    let mut softirq_sum = 0u64;
    let mut per_softirq_sums = [0u64; NR_SOFTIRQS];

    for cpu in snapshot.possible_cpus {
        let times = cpu_times(cpu);
        for (total, value) in total.iter_mut().zip(times) {
            *total += value;
        }
        irq_sum += cpu.irq_sum + cpu.arch_irq_stat;

        for (idx, value) in cpu.softirqs.iter().copied().enumerate() {
            per_softirq_sums[idx] += value;
            softirq_sum += value;
        }
    }
    irq_sum += snapshot.arch_irq_stat;

    push_cpu_times(&mut out, "cpu  ", &total);

    for (idx, cpu) in snapshot.possible_cpus.iter().enumerate() {
        if cpu.online {
            push_cpu_times(&mut out, &format!("cpu{} ", idx), &cpu_times(cpu));
        }
    }

    out.push_str(&format!("intr {}", irq_sum));
    for count in snapshot.irq_counts {
        out.push_str(&format!(" {}", count));
    }
    out.push('\n');

    out.push_str(&format!(
        "ctxt {}\nbtime {}\nprocesses {}\nprocs_running {}\nprocs_blocked {}\n",
        snapshot.context_switches,
        snapshot.boot_time_sec,
        snapshot.total_forks,
        snapshot.procs_running,
        snapshot.procs_blocked
    ));

    out.push_str(&format!("softirq {}", softirq_sum));
    for count in per_softirq_sums {
        out.push_str(&format!(" {}", count));
    }
    out.push('\n');
    out
}

pub fn show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let text = render_proc_stat(&DEFAULT_PROC_STAT_SNAPSHOT);
    super::util::copy_into(buf, &text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proc_stat_rendering_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/proc/stat.c"
        ));
        let kernel_stat = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/kernel_stat.h"
        ));
        let interrupt = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/interrupt.h"
        ));
        let time = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/time/time.c"
        ));

        assert!(source.contains("struct kernel_cpustat kcpustat;"));
        assert!(source.contains("kcpustat_cpu_fetch(&kcpustat, i);"));
        assert!(source.contains("show_irq_gap(p, i - next);"));
        assert!(source.contains("static int show_stat(struct seq_file *p, void *v)"));
        assert!(source.contains("for_each_possible_cpu(i)"));
        assert!(source.contains("for_each_online_cpu(i)"));
        assert!(source.contains("seq_put_decimal_ull(p, \"cpu  \", nsec_to_clock_t(user));"));
        assert!(source.contains("seq_printf(p, \"cpu%d\", i);"));
        assert!(source.contains("seq_put_decimal_ull(p, \"intr \", (unsigned long long)sum);"));
        assert!(source.contains("show_all_irqs(p);"));
        assert!(source.contains("nr_context_switches()"));
        assert!(source.contains("total_forks"));
        assert!(source.contains("nr_running()"));
        assert!(source.contains("nr_iowait()"));
        assert!(source.contains("seq_put_decimal_ull(p, \"softirq \","));
        assert!(source.contains("return single_open_size(file, show_stat, NULL, size);"));
        assert!(source.contains("proc_create(\"stat\", 0, NULL, &stat_proc_ops);"));
        assert!(kernel_stat.contains("enum cpu_usage_stat"));
        assert!(kernel_stat.contains("CPUTIME_GUEST_NICE"));
        assert!(interrupt.contains("NR_SOFTIRQS"));
        assert!(time.contains("u64 nsec_to_clock_t(u64 x)"));
        assert!(source.contains(STAT_PROC_OPS_SYMBOL));

        for (slot, target) in STAT_PROC_OPS {
            assert!(source.contains(slot));
            assert!(source.contains(target));
        }

        assert_eq!(CPUTIME_USER, 0);
        assert_eq!(CPUTIME_SOFTIRQ, 3);
        assert_eq!(CPUTIME_GUEST_NICE, 9);
        assert_eq!(NR_STATS, 10);
        assert_eq!(stat_open_size(2, 3), 1024 + 2 * 128 + 3 * 2);
    }

    #[test]
    fn default_show_uses_full_linux_proc_stat_line_set() {
        let rendered = render_proc_stat(&DEFAULT_PROC_STAT_SNAPSHOT);
        assert_eq!(
            rendered,
            "cpu  0 0 0 0 0 0 0 0 0 0\n\
             cpu0 0 0 0 0 0 0 0 0 0 0\n\
             intr 0\n\
             ctxt 0\n\
             btime 0\n\
             processes 1\n\
             procs_running 0\n\
             procs_blocked 0\n\
             softirq 0 0 0 0 0 0 0 0 0 0 0\n"
        );
    }

    #[test]
    fn render_proc_stat_aggregates_possible_and_online_cpus() {
        let mut cpu0 = ProcStatCpu::zero(true);
        cpu0.cpustat[CPUTIME_USER] = 10_000_000;
        cpu0.cpustat[CPUTIME_NICE] = 20_000_000;
        cpu0.cpustat[CPUTIME_SYSTEM] = 30_000_000;
        cpu0.cpustat[CPUTIME_IDLE] = 40_000_000;
        cpu0.cpustat[CPUTIME_IOWAIT] = 50_000_000;
        cpu0.cpustat[CPUTIME_IRQ] = 60_000_000;
        cpu0.cpustat[CPUTIME_SOFTIRQ] = 70_000_000;
        cpu0.cpustat[CPUTIME_STEAL] = 80_000_000;
        cpu0.cpustat[CPUTIME_GUEST] = 90_000_000;
        cpu0.cpustat[CPUTIME_GUEST_NICE] = 100_000_000;
        cpu0.idle_time_us = Some(45_000);
        cpu0.iowait_time_us = Some(55_000);
        cpu0.irq_sum = 7;
        cpu0.arch_irq_stat = 2;
        cpu0.softirqs[0] = 3;
        cpu0.softirqs[NR_SOFTIRQS - 1] = 5;

        let mut cpu1 = ProcStatCpu::zero(false);
        cpu1.cpustat[CPUTIME_USER] = 110_000_000;
        cpu1.cpustat[CPUTIME_IDLE] = 120_000_000;
        cpu1.cpustat[CPUTIME_IOWAIT] = 130_000_000;
        cpu1.irq_sum = 11;
        cpu1.softirqs[0] = 13;

        let cpus = [cpu0, cpu1];
        let irqs = [4, 0, 9];
        let snapshot = ProcStatSnapshot {
            possible_cpus: &cpus,
            irq_counts: &irqs,
            arch_irq_stat: 17,
            context_switches: 1234,
            boot_time_sec: 99,
            total_forks: 42,
            procs_running: 2,
            procs_blocked: 1,
        };

        let rendered = render_proc_stat(&snapshot);
        assert!(rendered.contains("cpu  12 2 3 16 18 6 7 8 9 10\n"));
        assert!(rendered.contains("cpu0 1 2 3 4 5 6 7 8 9 10\n"));
        assert!(!rendered.contains("cpu1 "));
        assert!(rendered.contains("intr 37 4 0 9\n"));
        assert!(rendered.contains("ctxt 1234\nbtime 99\nprocesses 42\n"));
        assert!(rendered.contains("procs_running 2\nprocs_blocked 1\n"));
        assert!(rendered.ends_with("softirq 21 16 0 0 0 0 0 0 0 0 5\n"));
    }

    #[test]
    fn idle_and_iowait_use_live_usec_values_only_for_online_cpus() {
        let mut online = ProcStatCpu::zero(true);
        online.cpustat[CPUTIME_IDLE] = 10;
        online.cpustat[CPUTIME_IOWAIT] = 20;
        online.idle_time_us = Some(3);
        online.iowait_time_us = Some(4);
        assert_eq!(get_idle_time(&online), 3 * NSEC_PER_USEC);
        assert_eq!(get_iowait_time(&online), 4 * NSEC_PER_USEC);

        let mut offline = online;
        offline.online = false;
        assert_eq!(get_idle_time(&offline), 10);
        assert_eq!(get_iowait_time(&offline), 20);
    }
}
