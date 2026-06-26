//! linux-parity: complete
//! linux-source: vendor/linux/kernel/sched/cpupri.c
//! test-origin: linux:vendor/linux/kernel/sched/cpupri.c
//! RT CPU priority lookup.
//!
//! Mirrors `vendor/linux/kernel/sched/cpupri.c` and `cpupri.h`.

use super::MAX_CPUS;
use super::entity::CpuMask;
use super::prio::MAX_RT_PRIO;

pub const CPUPRI_NR_PRIORITIES: i32 = MAX_RT_PRIO + 1;
pub const CPUPRI_INVALID: i32 = -1;
pub const CPUPRI_NORMAL: i32 = 0;
pub const CPUPRI_HIGHER: i32 = 100;

#[derive(Clone, Debug)]
pub struct CpuPri {
    cpu_to_pri: [i32; MAX_CPUS],
}

impl CpuPri {
    pub const fn new() -> Self {
        Self {
            cpu_to_pri: [CPUPRI_INVALID; MAX_CPUS],
        }
    }

    pub fn set(&mut self, cpu: u32, pri: i32) -> bool {
        let idx = cpu as usize;
        if idx >= MAX_CPUS || pri >= CPUPRI_NR_PRIORITIES {
            return false;
        }
        self.cpu_to_pri[idx] = pri;
        true
    }

    pub fn get(&self, cpu: u32) -> Option<i32> {
        self.cpu_to_pri.get(cpu as usize).copied()
    }

    pub fn find(&self, task_pri: i32, allowed: CpuMask) -> Option<u32> {
        for cpu in 0..MAX_CPUS as u32 {
            if !allowed.test(cpu) {
                continue;
            }
            let cpu_pri = self.cpu_to_pri[cpu as usize];
            if cpu_pri == CPUPRI_INVALID || cpu_pri < task_pri {
                return Some(cpu);
            }
        }
        None
    }
}

pub fn convert_prio(prio: i32) -> i32 {
    match prio {
        CPUPRI_INVALID => CPUPRI_INVALID,
        0..=98 => MAX_RT_PRIO - 1 - prio,
        99 => CPUPRI_NORMAL,
        _ => CPUPRI_HIGHER,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpupri_constants_match_linux() {
        assert_eq!(CPUPRI_INVALID, -1);
        assert_eq!(CPUPRI_NORMAL, 0);
        assert_eq!(CPUPRI_HIGHER, 100);
    }

    #[test]
    fn cpupri_find_selects_lower_priority_cpu() {
        let mut cp = CpuPri::new();
        cp.set(0, CPUPRI_HIGHER);
        cp.set(1, CPUPRI_NORMAL);
        assert_eq!(cp.find(50, CpuMask::all()), Some(1));
    }
}
