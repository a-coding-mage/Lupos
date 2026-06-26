//! linux-parity: complete
//! linux-source: vendor/linux/kernel/sched/build_utility.c
//! test-origin: linux:vendor/linux/kernel/sched/build_utility.c
//! Scheduler utility build aggregation.
//!
//! Mirrors `vendor/linux/kernel/sched/build_utility.c`. Linux includes common
//! scheduler support code through this unit; Lupos keeps an explicit inventory
//! for the utility modules that are built as first-class Rust modules.

pub const UTILITY_BUILD_INPUTS: &[&str] = &[
    "vendor/linux/kernel/sched/clock.c",
    "vendor/linux/kernel/sched/completion.c",
    "vendor/linux/kernel/sched/cpuacct.c",
    "vendor/linux/kernel/sched/cpudeadline.c",
    "vendor/linux/kernel/sched/cpufreq.c",
    "vendor/linux/kernel/sched/cpufreq_schedutil.c",
    "vendor/linux/kernel/sched/cpupri.c",
    "vendor/linux/kernel/sched/cputime.c",
    "vendor/linux/kernel/sched/debug.c",
    "vendor/linux/kernel/sched/isolation.c",
    "vendor/linux/kernel/sched/loadavg.c",
    "vendor/linux/kernel/sched/membarrier.c",
    "vendor/linux/kernel/sched/psi.c",
    "vendor/linux/kernel/sched/stats.c",
    "vendor/linux/kernel/sched/swait.c",
    "vendor/linux/kernel/sched/wait.c",
    "vendor/linux/kernel/sched/wait_bit.c",
];

pub fn utility_source_is_built(linux_path: &str) -> bool {
    UTILITY_BUILD_INPUTS.contains(&linux_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utility_inventory_includes_wait_and_membarrier() {
        assert!(utility_source_is_built("vendor/linux/kernel/sched/wait.c"));
        assert!(utility_source_is_built(
            "vendor/linux/kernel/sched/membarrier.c"
        ));
    }
}
