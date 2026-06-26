//! linux-parity: complete
//! linux-source: vendor/linux/kernel/sched/cpudeadline.c
//! test-origin: linux:vendor/linux/kernel/sched/cpudeadline.c
//! Deadline scheduler CPU selection.
//!
//! Mirrors `vendor/linux/kernel/sched/cpudeadline.c`. Linux keeps a heap of
//! CPUs ordered by earliest deadline; the small Lupos representation scans the
//! fixed CPU array and preserves the same "earliest active deadline" contract.

use super::MAX_CPUS;
use super::entity::CpuMask;

#[derive(Clone, Debug)]
pub struct CpuDeadline {
    deadlines: [u64; MAX_CPUS],
    active: CpuMask,
}

impl CpuDeadline {
    pub const fn new() -> Self {
        Self {
            deadlines: [u64::MAX; MAX_CPUS],
            active: CpuMask::empty(),
        }
    }

    pub fn set(&mut self, cpu: u32, deadline: u64) -> bool {
        let idx = cpu as usize;
        if idx >= MAX_CPUS {
            return false;
        }
        self.deadlines[idx] = deadline;
        self.active.set(cpu);
        true
    }

    pub fn clear(&mut self, cpu: u32) {
        let idx = cpu as usize;
        if idx < MAX_CPUS {
            self.deadlines[idx] = u64::MAX;
            self.active.clear(cpu);
        }
    }

    pub fn earliest_cpu(&self, allowed: CpuMask) -> Option<u32> {
        let mut best_cpu = None;
        let mut best_deadline = u64::MAX;
        for cpu in 0..MAX_CPUS as u32 {
            if !self.active.test(cpu) || !allowed.test(cpu) {
                continue;
            }
            let deadline = self.deadlines[cpu as usize];
            if deadline < best_deadline {
                best_deadline = deadline;
                best_cpu = Some(cpu);
            }
        }
        best_cpu
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpudeadline_picks_earliest_allowed_cpu() {
        let mut dl = CpuDeadline::new();
        dl.set(0, 100);
        dl.set(1, 50);
        assert_eq!(dl.earliest_cpu(CpuMask::all()), Some(1));
        assert_eq!(dl.earliest_cpu(CpuMask::one(0)), Some(0));
    }
}
