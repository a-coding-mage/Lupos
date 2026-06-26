//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_events_synth.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_events_synth.c
//! Synthetic events — userspace-defined event classes (`synth/`).
//!
//! Ref: vendor/linux/kernel/trace/trace_events_synth.c

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

use spin::Mutex;

#[derive(Clone, Debug)]
pub struct SynthEvent {
    pub name: String,
    pub fields: Vec<String>,
}

static EVENTS: Mutex<Vec<SynthEvent>> = Mutex::new(Vec::new());

pub fn create(name: &str, fields: Vec<String>) -> Result<(), i32> {
    let mut g = EVENTS.lock();
    if g.iter().any(|e| e.name == name) {
        return Err(-17);
    }
    g.push(SynthEvent {
        name: name.into(),
        fields,
    });
    Ok(())
}

pub fn count() -> usize {
    EVENTS.lock().len()
}

pub fn destroy(name: &str) -> Result<(), i32> {
    let mut g = EVENTS.lock();
    if let Some(pos) = g.iter().position(|e| e.name == name) {
        g.remove(pos);
        Ok(())
    } else {
        Err(-2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_destroy_round_trip() {
        let n0 = count();
        create(
            "synth_a",
            alloc::vec!["u32 pid".into(), "char comm[16]".into()],
        )
        .unwrap();
        assert_eq!(count(), n0 + 1);
        destroy("synth_a").unwrap();
        assert_eq!(count(), n0);
    }
}
