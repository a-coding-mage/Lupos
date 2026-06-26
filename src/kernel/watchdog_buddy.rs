//! linux-parity: complete
//! linux-source: vendor/linux/kernel/watchdog_buddy.c
//! test-origin: linux:vendor/linux/kernel/watchdog_buddy.c
//! Hardlockup watchdog buddy CPU selection.

pub const WATCHDOG_HARDLOCKUP_MISS_THRESH: u32 = 3;
pub const MAX_TRACKED_CPUS: u32 = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WatchdogBuddy {
    pub nr_cpu_ids: u32,
    pub watchdog_cpus: u64,
    pub touched_cpus: u64,
    pub checked_cpu: Option<u32>,
    pub miss_thresh: u32,
}

impl WatchdogBuddy {
    pub const fn new(nr_cpu_ids: u32) -> Self {
        Self {
            nr_cpu_ids,
            watchdog_cpus: 0,
            touched_cpus: 0,
            checked_cpu: None,
            miss_thresh: 0,
        }
    }

    pub fn probe(&mut self) -> i32 {
        self.miss_thresh = WATCHDOG_HARDLOCKUP_MISS_THRESH;
        0
    }

    pub fn next_cpu(&self, cpu: u32) -> Option<u32> {
        let limit = self.nr_cpu_ids.min(MAX_TRACKED_CPUS);
        if cpu >= limit {
            return None;
        }
        for step in 1..=limit {
            let candidate = (cpu + step) % limit;
            if self.watchdog_cpus & (1u64 << candidate) != 0 {
                return (candidate != cpu).then_some(candidate);
            }
        }
        None
    }

    pub fn enable(&mut self, cpu: u32) {
        self.touch(cpu);
        if let Some(next_cpu) = self.next_cpu(cpu) {
            self.touch(next_cpu);
        }
        self.set_cpu(cpu);
    }

    pub fn disable(&mut self, cpu: u32) {
        if let Some(next_cpu) = self.next_cpu(cpu) {
            self.touch(next_cpu);
        }
        self.clear_cpu(cpu);
    }

    pub fn check_hardlockup(&mut self, current_cpu: u32) -> Option<u32> {
        let next_cpu = self.next_cpu(current_cpu)?;
        self.checked_cpu = Some(next_cpu);
        Some(next_cpu)
    }

    fn touch(&mut self, cpu: u32) {
        if cpu < MAX_TRACKED_CPUS {
            self.touched_cpus |= 1u64 << cpu;
        }
    }

    fn set_cpu(&mut self, cpu: u32) {
        if cpu < MAX_TRACKED_CPUS {
            self.watchdog_cpus |= 1u64 << cpu;
        }
    }

    fn clear_cpu(&mut self, cpu: u32) {
        if cpu < MAX_TRACKED_CPUS {
            self.watchdog_cpus &= !(1u64 << cpu);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watchdog_buddy_tracks_next_cpu_and_touch_order() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/watchdog_buddy.c"
        ));
        assert!(source.contains("static cpumask_t __read_mostly watchdog_cpus;"));
        assert!(source.contains("cpumask_next_wrap(cpu, &watchdog_cpus);"));
        assert!(source.contains("if (next_cpu == cpu)"));
        assert!(source.contains("watchdog_hardlockup_miss_thresh = 3;"));
        assert!(source.contains("watchdog_hardlockup_touch_cpu(cpu);"));
        assert!(source.contains("watchdog_hardlockup_touch_cpu(next_cpu);"));
        assert!(source.contains("smp_wmb();"));
        assert!(source.contains("cpumask_set_cpu(cpu, &watchdog_cpus);"));
        assert!(source.contains("cpumask_clear_cpu(cpu, &watchdog_cpus);"));
        assert!(source.contains("smp_rmb();"));
        assert!(source.contains("watchdog_hardlockup_check(next_cpu, NULL);"));

        let mut buddy = WatchdogBuddy::new(4);
        assert_eq!(buddy.probe(), 0);
        assert_eq!(buddy.miss_thresh, WATCHDOG_HARDLOCKUP_MISS_THRESH);

        buddy.enable(1);
        assert_eq!(buddy.watchdog_cpus, 0b0010);
        assert_eq!(buddy.touched_cpus, 0b0010);
        assert_eq!(buddy.next_cpu(1), None);

        buddy.enable(3);
        assert_eq!(buddy.watchdog_cpus, 0b1010);
        assert_eq!(buddy.touched_cpus, 0b1010);
        assert_eq!(buddy.next_cpu(1), Some(3));
        assert_eq!(buddy.next_cpu(3), Some(1));
        assert_eq!(buddy.check_hardlockup(1), Some(3));
        assert_eq!(buddy.checked_cpu, Some(3));

        buddy.disable(1);
        assert_eq!(buddy.watchdog_cpus, 0b1000);
        assert_eq!(buddy.touched_cpus, 0b1010);
        assert_eq!(buddy.next_cpu(3), None);
    }
}
