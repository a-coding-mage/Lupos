//! linux-parity: partial
//! linux-source: vendor/linux/kernel/trace
//! test-origin: linux:vendor/linux/kernel/trace
//! kprobes — `int3` based instruction-level instrumentation.
//!
//! Mirrors `vendor/linux/kernel/kprobes.c::register_kprobe`.
//!
//! Registration prepares a displaced-instruction slot, atomically arms the
//! target with INT3 through the x86 text-poke backend, and keeps the slot alive
//! until every in-flight #DB completion has returned to the original stream.

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use spin::Mutex;

use crate::arch::x86::kernel::kprobes::core::{
    KprobeExecution, KprobeExecutionBehavior, LiveKprobe, arm_live_kprobe, disarm_live_kprobe,
    free_live_kprobe, prepare_live_kprobe,
};
use crate::arch::x86::lib::insn::MAX_INSN_SIZE;

use super::ring_buffer::{TRACE_KPROBE, TRACE_RB, TraceEvent};

pub type KprobePreFn = fn(addr: u64, data: usize);
pub type KprobePostFn = fn(addr: u64, data: usize);

pub struct Kprobe {
    pub addr: u64,
    pub data: usize,
    pub pre: Option<KprobePreFn>,
    pub post: Option<KprobePostFn>,
    pub enabled: AtomicBool,
}

impl Kprobe {
    pub const fn new(addr: u64) -> Self {
        Self {
            addr,
            data: 0,
            pre: None,
            post: None,
            enabled: AtomicBool::new(false),
        }
    }
}

struct KprobeRegistry {
    probes: Vec<RegisteredKprobe>,
}

struct RegisteredKprobe {
    probe: &'static Kprobe,
    actual_addr: u64,
    #[cfg(not(test))]
    live: LiveKprobe,
}

unsafe impl Send for RegisteredKprobe {}

impl KprobeRegistry {
    const fn new() -> Self {
        Self { probes: Vec::new() }
    }
}

static KPROBE_REGISTRY: Mutex<KprobeRegistry> = Mutex::new(KprobeRegistry::new());
static KPROBE_UPDATE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AddressRange {
    pub start: usize,
    pub end: usize,
}

impl AddressRange {
    pub fn from_start_size(start: usize, size: usize) -> Option<Self> {
        if start == 0 || size == 0 {
            return None;
        }
        Some(Self {
            start,
            end: start.checked_add(size)?,
        })
    }

    const fn contains(self, address: usize) -> bool {
        address >= self.start && address < self.end
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ModuleBlacklistEntry {
    owner: usize,
    range: AddressRange,
}

static KPROBE_BLACKLIST: Mutex<Vec<ModuleBlacklistEntry>> = Mutex::new(Vec::new());

pub fn register_kprobe(kp: &'static Kprobe) -> Result<(), i32> {
    let _update = KPROBE_UPDATE_LOCK.lock();
    if within_kprobe_blacklist(kp.addr as usize) {
        return Err(-22); // EINVAL
    }
    let mut g = KPROBE_REGISTRY.lock();
    if g.probes.iter().any(|record| record.probe.addr == kp.addr) {
        return Err(-17); // EEXIST
    }

    #[cfg(not(test))]
    {
        let live = prepare_live_kprobe(kp.addr).map_err(|error| -error.abs())?;
        let actual_addr = live.arch.addr;
        if g.probes
            .iter()
            .any(|record| record.actual_addr == actual_addr)
        {
            free_live_kprobe(live);
            return Err(-17);
        }
        g.probes.push(RegisteredKprobe {
            probe: kp,
            actual_addr,
            live,
        });
        let index = g.probes.len() - 1;
        // Publish the registry record before INT3 so every possible trap can
        // resolve it. No trap is possible before arm_live_kprobe writes INT3.
        kp.enabled.store(true, Ordering::Release);
        let live = core::ptr::addr_of_mut!(g.probes[index].live);
        drop(g);
        let arm_result = unsafe { arm_live_kprobe(&mut *live) };
        g = KPROBE_REGISTRY.lock();
        if let Err(error) = arm_result {
            kp.enabled.store(false, Ordering::Release);
            let Some(index) = g
                .probes
                .iter()
                .position(|record| record.probe.addr == kp.addr)
            else {
                return Err(-error.abs());
            };
            let record = g.probes.swap_remove(index);
            free_live_kprobe(record.live);
            return Err(-error.abs());
        }
    }
    #[cfg(test)]
    g.probes.push(RegisteredKprobe {
        probe: kp,
        actual_addr: kp.addr,
    });
    kp.enabled.store(true, Ordering::Release);
    Ok(())
}

pub fn unregister_kprobe(addr: u64) -> Result<(), i32> {
    let _update = KPROBE_UPDATE_LOCK.lock();
    let mut g = KPROBE_REGISTRY.lock();
    let Some(index) = g.probes.iter().position(|record| record.probe.addr == addr) else {
        return Err(-2); // ENOENT
    };

    #[cfg(not(test))]
    {
        let live = core::ptr::addr_of_mut!(g.probes[index].live);
        drop(g);
        unsafe { disarm_live_kprobe(&mut *live) }.map_err(|error| -error.abs())?;
        while crate::arch::x86::kernel::kprobes::core::kprobe_handlers_active() {
            core::hint::spin_loop();
        }
        g = KPROBE_REGISTRY.lock();
        let Some(index) = g.probes.iter().position(|record| record.probe.addr == addr) else {
            return Err(-2);
        };
        g.probes[index]
            .probe
            .enabled
            .store(false, Ordering::Release);
        let record = g.probes.swap_remove(index);
        free_live_kprobe(record.live);
    }
    #[cfg(test)]
    {
        g.probes[index]
            .probe
            .enabled
            .store(false, Ordering::Release);
        g.probes.swap_remove(index);
    }
    Ok(())
}

/// Manually invoke a kprobe by address.  Used by both the IDT `on_breakpoint`
/// hook (deferred until text patching lands) and by direct test/instrumentation
/// call sites.  Returns true if a probe fired.
pub fn fire_kprobe(addr: u64) -> bool {
    if begin_kprobe(addr).is_none() {
        return false;
    }
    finish_kprobe(addr);
    true
}

/// Resolve one armed probe and run its pre-handler without retaining the
/// registry lock across callback code.
pub(crate) fn begin_kprobe(addr: u64) -> Option<KprobeExecution> {
    let (probe, execution) = {
        let registry = KPROBE_REGISTRY.lock();
        let record = registry.probes.iter().find(|record| {
            record.actual_addr == addr && record.probe.enabled.load(Ordering::Acquire)
        })?;
        #[cfg(not(test))]
        let execution = record.live.execution();
        #[cfg(test)]
        let execution = KprobeExecution {
            original_ip: record.actual_addr,
            slot_ip: record.actual_addr,
            instruction_len: 1,
            bytes: [0u8; MAX_INSN_SIZE],
            behavior: KprobeExecutionBehavior::default(),
        };
        (record.probe, execution)
    };
    if let Some(handler) = probe.pre {
        handler(probe.addr, probe.data);
    }
    TRACE_RB.push(TraceEvent {
        ts_nsec: crate::kernel::time::jiffies::jiffies() as u64 * 1_000_000,
        ev_type: TRACE_KPROBE,
        cpu: crate::kernel::sched::current_cpu() as u16,
        pid: 0,
        arg0: probe.addr,
        arg1: 0,
    });
    Some(execution)
}

pub(crate) fn finish_kprobe(addr: u64) {
    let probe = {
        let registry = KPROBE_REGISTRY.lock();
        registry
            .probes
            .iter()
            .find(|record| record.actual_addr == addr)
            .map(|record| record.probe)
    };
    if let Some(probe) = probe
        && let Some(handler) = probe.post
    {
        handler(probe.addr, probe.data);
    }
}

pub fn registered_count() -> usize {
    KPROBE_REGISTRY.lock().probes.len()
}

/// Match `kernel/kprobes.c::__within_kprobe_blacklist()` for module-owned
/// `_kprobe_blacklist`, `.kprobes.text`, and `.noinstr.text` entries.
pub fn within_kprobe_blacklist(address: usize) -> bool {
    KPROBE_BLACKLIST
        .lock()
        .iter()
        .any(|entry| entry.range.contains(address))
}

/// Module `COMING` callback from `kprobes_module_callback()`.
pub fn module_coming(
    owner: usize,
    symbol_blacklist: &[usize],
    kprobes_text: Option<AddressRange>,
    noinstr_text: Option<AddressRange>,
) -> Result<(), i32> {
    let mut blacklist = KPROBE_BLACKLIST.lock();
    if blacklist.iter().any(|entry| entry.owner == owner) {
        return Err(-17); // EEXIST
    }

    blacklist.extend(
        symbol_blacklist
            .iter()
            .copied()
            .filter(|address| *address != 0)
            .map(|address| ModuleBlacklistEntry {
                owner,
                range: AddressRange {
                    start: address,
                    end: address.saturating_add(1),
                },
            }),
    );
    for range in [kprobes_text, noinstr_text].into_iter().flatten() {
        blacklist.push(ModuleBlacklistEntry { owner, range });
    }
    Ok(())
}

fn kill_probes_in_ranges(ranges: &[AddressRange]) -> Result<(), i32> {
    if ranges.is_empty() {
        return Ok(());
    }

    let _update = KPROBE_UPDATE_LOCK.lock();
    #[cfg(not(test))]
    {
        let mut probes = KPROBE_REGISTRY.lock();
        let mut matches = Vec::new();
        for record in probes.probes.iter_mut() {
            if record.probe.enabled.load(Ordering::Acquire)
                && ranges
                    .iter()
                    .any(|range| range.contains(record.actual_addr as usize))
            {
                matches.push((record.actual_addr, core::ptr::addr_of_mut!(record.live)));
            }
        }
        drop(probes);

        // Keep every record published and enabled until its INT3 is gone;
        // a CPU which raced with the text poke must still be able to resolve
        // and finish the probe. KPROBE_UPDATE_LOCK keeps the Vec stable while
        // the registry lock is deliberately released around cross-CPU sync.
        let mut disarmed = 0usize;
        for (_, live) in matches.iter().copied() {
            if let Err(error) = unsafe { disarm_live_kprobe(&mut *live) } {
                // Preserve the all-armed state on a partial text-poke failure;
                // leaving an enabled registry record with restored text would
                // make its lifecycle indistinguishable from a live probe.
                for (_, previous) in matches[..disarmed].iter().copied() {
                    let _ = unsafe { arm_live_kprobe(&mut *previous) };
                }
                return Err(-error.abs());
            }
            disarmed += 1;
        }
        while crate::arch::x86::kernel::kprobes::core::kprobe_handlers_active() {
            core::hint::spin_loop();
        }
        let probes = KPROBE_REGISTRY.lock();
        for (address, _) in matches {
            if let Some(record) = probes
                .probes
                .iter()
                .find(|record| record.actual_addr == address)
            {
                // Linux's kill_kprobe() leaves the registration present but
                // marks it gone/disarmed so later unregister is well-defined.
                record.probe.enabled.store(false, Ordering::Release);
            }
        }
    }
    #[cfg(test)]
    {
        let probes = KPROBE_REGISTRY.lock();
        for record in probes.probes.iter() {
            if ranges
                .iter()
                .any(|range| range.contains(record.actual_addr as usize))
            {
                record.probe.enabled.store(false, Ordering::Release);
            }
        }
    }
    Ok(())
}

/// Module `LIVE` callback.  At this transition Linux is about to free only
/// the module init layout, so probes into init text must be killed.
pub fn module_live(_owner: usize, init_text: &[AddressRange]) -> Result<(), i32> {
    kill_probes_in_ranges(init_text)
}

/// Module `GOING` callback.  Kill probes into both layouts before releasing
/// the blacklist entries whose backing addresses belong to the module.
pub fn module_going(
    owner: usize,
    core_text: &[AddressRange],
    init_text: &[AddressRange],
) -> Result<(), i32> {
    kill_probes_in_ranges(init_text)?;
    kill_probes_in_ranges(core_text)?;
    KPROBE_BLACKLIST.lock().retain(|entry| entry.owner != owner);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_KP: Kprobe = Kprobe {
        addr: 0xdead_beef,
        data: 42,
        pre: Some(my_pre),
        post: Some(my_post),
        enabled: AtomicBool::new(false),
    };

    static PRE_HITS: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
    static POST_HITS: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);

    fn my_pre(_addr: u64, data: usize) {
        assert_eq!(data, 42);
        PRE_HITS.fetch_add(1, Ordering::Relaxed);
    }

    fn my_post(_addr: u64, data: usize) {
        assert_eq!(data, 42);
        POST_HITS.fetch_add(1, Ordering::Relaxed);
    }

    #[test]
    fn register_fire_unregister_round_trip() {
        PRE_HITS.store(0, Ordering::Relaxed);
        POST_HITS.store(0, Ordering::Relaxed);

        register_kprobe(&TEST_KP).unwrap();
        // Duplicate registration → EEXIST.
        assert_eq!(register_kprobe(&TEST_KP), Err(-17));

        assert!(fire_kprobe(TEST_KP.addr));
        assert!(fire_kprobe(TEST_KP.addr));
        assert_eq!(PRE_HITS.load(Ordering::Relaxed), 2);
        assert_eq!(POST_HITS.load(Ordering::Relaxed), 2);

        // Wrong address → no fire.
        assert!(!fire_kprobe(0xcafe));

        unregister_kprobe(TEST_KP.addr).unwrap();
        // After unregister, fire is a no-op.
        assert!(!fire_kprobe(TEST_KP.addr));
        // Re-unregister → ENOENT.
        assert_eq!(unregister_kprobe(TEST_KP.addr), Err(-2));
    }
}
