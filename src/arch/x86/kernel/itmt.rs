//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/itmt.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/itmt.c
//! Intel Turbo Boost Max Technology 3.0 (ITMT) scheduler integration.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/itmt.c
//!
//! On platforms with ITMT, a subset of cores can boost to higher turbo
//! frequencies than the rest. The pstate driver detects this and calls
//! `sched_set_itmt_support()` + `sched_set_itmt_core_prio()`, which
//! makes the scheduler prefer high-priority cores. The debugfs control
//! `x86/sched_itmt_enabled` toggles the optimisation at runtime.

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::ENOMEM;

/// Container mirroring the per-CPU `sched_core_priority` array.
#[derive(Debug, Clone)]
pub struct ItmtState {
    pub core_priorities: Vec<i32>,
    pub itmt_capable: bool,
    pub itmt_enabled: bool,
    pub topology_dirty: bool,
    pub debugfs_files: Vec<&'static str>,
    pub rebuild_count: u32,
}

impl ItmtState {
    pub fn new(nr_cpus: usize) -> Self {
        Self {
            core_priorities: alloc::vec![0; nr_cpus],
            itmt_capable: false,
            itmt_enabled: false,
            topology_dirty: false,
            debugfs_files: Vec::new(),
            rebuild_count: 0,
        }
    }
}

/// Trait seam for `rebuild_sched_domains()` — production hooks the real
/// scheduler-domain rebuilder.
pub trait SchedDomains {
    fn rebuild(&self, state: &mut ItmtState);
}

/// Default backend that just bumps the rebuild count. Used in tests.
pub struct RecordingSched;

impl SchedDomains for RecordingSched {
    fn rebuild(&self, state: &mut ItmtState) {
        state.rebuild_count += 1;
    }
}

/// `sched_set_itmt_support` — indicate the platform is ITMT-capable.
/// Idempotent: returns Ok(()) without re-creating the debugfs files if
/// already capable.
pub fn sched_set_itmt_support<S: SchedDomains>(
    state: &mut ItmtState,
    sched: &S,
) -> Result<(), i32> {
    if state.itmt_capable {
        return Ok(());
    }
    state.debugfs_files.push("sched_itmt_enabled");
    state.debugfs_files.push("sched_core_priority");
    state.itmt_capable = true;
    state.itmt_enabled = true;
    state.topology_dirty = true;
    sched.rebuild(state);
    Ok(())
}

/// `sched_clear_itmt_support` — revoke ITMT capability. If the toggle
/// was on, also rebuild domains to drop the bias.
pub fn sched_clear_itmt_support<S: SchedDomains>(state: &mut ItmtState, sched: &S) {
    if !state.itmt_capable {
        return;
    }
    state.itmt_capable = false;
    state.debugfs_files.clear();
    if state.itmt_enabled {
        state.itmt_enabled = false;
        state.topology_dirty = true;
        sched.rebuild(state);
    }
}

/// `sched_set_itmt_core_prio` — set the priority of one CPU.
pub fn sched_set_itmt_core_prio(state: &mut ItmtState, prio: i32, cpu: usize) -> Result<(), i32> {
    if cpu >= state.core_priorities.len() {
        return Err(ENOMEM);
    }
    state.core_priorities[cpu] = prio;
    Ok(())
}

/// `arch_asym_cpu_priority` — getter.
pub fn arch_asym_cpu_priority(state: &ItmtState, cpu: usize) -> i32 {
    state.core_priorities.get(cpu).copied().unwrap_or(0)
}

/// `sched_itmt_enabled_write` — the debugfs `write` hook. Returns the new
/// enabled state and whether the value changed (which mirrors Linux's
/// "rebuild only on change" behaviour).
pub fn sched_itmt_enabled_write<S: SchedDomains>(
    state: &mut ItmtState,
    sched: &S,
    new_value: bool,
) -> bool {
    let changed = state.itmt_enabled != new_value;
    state.itmt_enabled = new_value;
    if changed {
        state.topology_dirty = true;
        sched.rebuild(state);
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_state_initialises_priorities_to_zero() {
        let s = ItmtState::new(4);
        assert_eq!(s.core_priorities, alloc::vec![0, 0, 0, 0]);
        assert!(!s.itmt_capable);
        assert!(!s.itmt_enabled);
    }

    #[test]
    fn set_support_creates_files_and_rebuilds_once() {
        let mut s = ItmtState::new(4);
        sched_set_itmt_support(&mut s, &RecordingSched).unwrap();
        assert!(s.itmt_capable);
        assert!(s.itmt_enabled);
        assert!(s.topology_dirty);
        assert_eq!(s.rebuild_count, 1);
        assert!(s.debugfs_files.contains(&"sched_itmt_enabled"));
        assert!(s.debugfs_files.contains(&"sched_core_priority"));
    }

    #[test]
    fn set_support_is_idempotent() {
        let mut s = ItmtState::new(4);
        sched_set_itmt_support(&mut s, &RecordingSched).unwrap();
        sched_set_itmt_support(&mut s, &RecordingSched).unwrap();
        assert_eq!(s.debugfs_files.len(), 2);
        assert_eq!(s.rebuild_count, 1);
    }

    #[test]
    fn clear_support_when_enabled_triggers_rebuild() {
        let mut s = ItmtState::new(4);
        sched_set_itmt_support(&mut s, &RecordingSched).unwrap();
        sched_clear_itmt_support(&mut s, &RecordingSched);
        assert!(!s.itmt_capable);
        assert!(!s.itmt_enabled);
        assert_eq!(s.rebuild_count, 2);
    }

    #[test]
    fn clear_support_when_already_disabled_is_noop() {
        let mut s = ItmtState::new(4);
        sched_clear_itmt_support(&mut s, &RecordingSched);
        assert_eq!(s.rebuild_count, 0);
    }

    #[test]
    fn set_core_prio_updates_per_cpu_array() {
        let mut s = ItmtState::new(4);
        sched_set_itmt_core_prio(&mut s, 17, 2).unwrap();
        assert_eq!(arch_asym_cpu_priority(&s, 2), 17);
    }

    #[test]
    fn set_core_prio_rejects_out_of_range() {
        let mut s = ItmtState::new(4);
        assert_eq!(sched_set_itmt_core_prio(&mut s, 1, 100), Err(ENOMEM));
    }

    #[test]
    fn enabled_write_only_rebuilds_on_change() {
        let mut s = ItmtState::new(4);
        sched_set_itmt_support(&mut s, &RecordingSched).unwrap();
        let before = s.rebuild_count;
        let changed = sched_itmt_enabled_write(&mut s, &RecordingSched, true);
        assert!(!changed);
        assert_eq!(s.rebuild_count, before);
        let changed = sched_itmt_enabled_write(&mut s, &RecordingSched, false);
        assert!(changed);
        assert_eq!(s.rebuild_count, before + 1);
    }
}
