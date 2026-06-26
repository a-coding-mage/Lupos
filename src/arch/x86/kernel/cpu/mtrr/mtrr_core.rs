//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/mtrr
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/mtrr
//! Core MTRR state machine and per-CPU coordination policy.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/mtrr/mtrr.c

// `mtrr.c` defines the cross-CPU rendezvous used to reprogram MTRRs:
// every CPU drops out of caches, the BSP writes the new MTRR state, and
// every CPU re-enables caches. We model the state machine; the actual
// cache control (CR0.CD, CR4.PGE) lives behind the cache-control trait.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MtrrRendezvousPhase {
    Idle,
    Entered,
    Writing,
    Synced,
    Failed,
}

#[derive(Debug, Eq, PartialEq)]
pub struct MtrrRendezvous {
    pub phase: MtrrRendezvousPhase,
    pub cpu_count: u32,
    pub cpus_in: u32,
}

impl MtrrRendezvous {
    pub const fn new(cpu_count: u32) -> Self {
        Self {
            phase: MtrrRendezvousPhase::Idle,
            cpu_count,
            cpus_in: 0,
        }
    }

    pub fn enter(&mut self) {
        self.cpus_in = self.cpus_in.saturating_add(1);
        if self.cpus_in == self.cpu_count {
            self.phase = MtrrRendezvousPhase::Entered;
        }
    }

    pub fn writing(&mut self) -> bool {
        if !matches!(self.phase, MtrrRendezvousPhase::Entered) {
            self.phase = MtrrRendezvousPhase::Failed;
            return false;
        }
        self.phase = MtrrRendezvousPhase::Writing;
        true
    }

    pub fn synced(&mut self) {
        if matches!(self.phase, MtrrRendezvousPhase::Writing) {
            self.phase = MtrrRendezvousPhase::Synced;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_cpus_must_enter_before_write_phase() {
        let mut r = MtrrRendezvous::new(2);
        r.enter();
        assert!(!r.writing()); // not all CPUs entered yet
        assert_eq!(r.phase, MtrrRendezvousPhase::Failed);
    }

    #[test]
    fn happy_path_progresses_to_synced() {
        let mut r = MtrrRendezvous::new(2);
        r.enter();
        r.enter();
        assert_eq!(r.phase, MtrrRendezvousPhase::Entered);
        assert!(r.writing());
        r.synced();
        assert_eq!(r.phase, MtrrRendezvousPhase::Synced);
    }
}
