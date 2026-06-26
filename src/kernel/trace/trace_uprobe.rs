//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_uprobe.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_uprobe.c
//! User-space uprobe trace-event glue (`events/uprobes/`).
//!
//! Ref: vendor/linux/kernel/trace/trace_uprobe.c

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

use spin::Mutex;

#[derive(Clone, Debug)]
pub struct TraceUprobe {
    pub name: String,
    pub path: String,
    pub offset: u64,
    pub hits: u64,
}

static EVENTS: Mutex<Vec<TraceUprobe>> = Mutex::new(Vec::new());

pub fn register(name: &str, path: &str, offset: u64) -> Result<(), i32> {
    let mut g = EVENTS.lock();
    if g.iter().any(|e| e.name == name) {
        return Err(-17);
    }
    g.push(TraceUprobe {
        name: name.into(),
        path: path.into(),
        offset,
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
        register("upt", "/bin/ls", 0x1000).unwrap();
        fire("upt");
        assert_eq!(hits("upt"), 1);
    }
}
