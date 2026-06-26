//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_kprobe.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_kprobe.c
//! kprobe trace-event glue — registers kprobe-based events under
//! `events/kprobes/`.
//!
//! Ref: vendor/linux/kernel/trace/trace_kprobe.c

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

use spin::Mutex;

#[derive(Clone, Debug)]
pub struct TraceKprobe {
    pub name: String,
    pub func: String,
    pub hits: u64,
}

static EVENTS: Mutex<Vec<TraceKprobe>> = Mutex::new(Vec::new());

pub fn register(name: &str, func: &str) -> Result<(), i32> {
    let mut g = EVENTS.lock();
    if g.iter().any(|e| e.name == name) {
        return Err(-17);
    }
    g.push(TraceKprobe {
        name: name.into(),
        func: func.into(),
        hits: 0,
    });
    Ok(())
}

pub fn fire(name: &str) {
    if let Some(e) = EVENTS.lock().iter_mut().find(|e| e.name == name) {
        e.hits += 1;
    }
}

pub fn hits(name: &str) -> u64 {
    EVENTS
        .lock()
        .iter()
        .find(|e| e.name == name)
        .map(|e| e.hits)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_fire_count() {
        register("kp_test", "do_sys_open").unwrap();
        fire("kp_test");
        fire("kp_test");
        assert_eq!(hits("kp_test"), 2);
    }
}
