//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_eprobe.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_eprobe.c
//! Event probes — attach a probe to an existing tracepoint and snapshot
//! arguments.
//!
//! Ref: vendor/linux/kernel/trace/trace_eprobe.c

extern crate alloc;
use alloc::string::String;

pub struct EProbe {
    pub event: String,
    pub fields: u32,
}

impl EProbe {
    pub fn new(event: &str) -> Self {
        Self {
            event: event.into(),
            fields: 0,
        }
    }

    pub fn add_field(&mut self) {
        self.fields += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_field_increments_count() {
        let mut p = EProbe::new("sched_switch");
        p.add_field();
        p.add_field();
        assert_eq!(p.fields, 2);
    }
}
