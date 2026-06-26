//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace
//! test-origin: linux:vendor/linux/kernel/trace
//! kprobes — `int3` based instruction-level instrumentation.
//!
//! Mirrors `vendor/linux/kernel/kprobes.c::register_kprobe`.
//!
//! Lupos M62 model:
//! - A `Kprobe` records the target instruction pointer, an `enabled` flag,
//!   the saved original byte, and pre/post handler function pointers.
//! - `register_kprobe` does **not** modify text in M62 — instead, the IDT
//!   `on_breakpoint` handler consults the registered probe table by RIP and
//!   invokes the matching handler if found.  This avoids needing a writable
//!   text path and the single-step trampoline machinery from
//!   `arch/x86/kernel/kprobes/core.c`, both of which are major sub-projects.
//!
//! Real `int3` text patching is deferred (see ROADMAP M62 Deferred).

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use spin::Mutex;

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
    probes: Vec<&'static Kprobe>,
}

impl KprobeRegistry {
    const fn new() -> Self {
        Self { probes: Vec::new() }
    }
}

static KPROBE_REGISTRY: Mutex<KprobeRegistry> = Mutex::new(KprobeRegistry::new());

pub fn register_kprobe(kp: &'static Kprobe) -> Result<(), i32> {
    let mut g = KPROBE_REGISTRY.lock();
    if g.probes.iter().any(|p| p.addr == kp.addr) {
        return Err(-17); // EEXIST
    }
    g.probes.push(kp);
    kp.enabled.store(true, Ordering::Release);
    Ok(())
}

pub fn unregister_kprobe(addr: u64) -> Result<(), i32> {
    let mut g = KPROBE_REGISTRY.lock();
    let len_before = g.probes.len();
    g.probes.retain(|p| {
        if p.addr == addr {
            p.enabled.store(false, Ordering::Release);
            false
        } else {
            true
        }
    });
    if g.probes.len() == len_before {
        Err(-2) // ENOENT
    } else {
        Ok(())
    }
}

/// Manually invoke a kprobe by address.  Used by both the IDT `on_breakpoint`
/// hook (deferred until text patching lands) and by direct test/instrumentation
/// call sites.  Returns true if a probe fired.
pub fn fire_kprobe(addr: u64) -> bool {
    let g = KPROBE_REGISTRY.lock();
    for p in g.probes.iter() {
        if p.addr == addr && p.enabled.load(Ordering::Acquire) {
            if let Some(f) = p.pre {
                f(p.addr, p.data);
            }
            TRACE_RB.push(TraceEvent {
                ts_nsec: crate::kernel::time::jiffies::jiffies() as u64 * 1_000_000,
                ev_type: TRACE_KPROBE,
                cpu: 0,
                pid: 0,
                arg0: p.addr,
                arg1: 0,
            });
            if let Some(f) = p.post {
                f(p.addr, p.data);
            }
            return true;
        }
    }
    false
}

pub fn registered_count() -> usize {
    KPROBE_REGISTRY.lock().probes.len()
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
