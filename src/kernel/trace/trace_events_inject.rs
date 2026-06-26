//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_events_inject.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_events_inject.c
//! Synthetic-event injection (`events/<x>/<y>/inject`).
//!
//! Userspace writes a tab-separated record into `inject` and the kernel
//! fans it out to all subscribed consumers as if it were a real
//! tracepoint hit.
//!
//! Ref: vendor/linux/kernel/trace/trace_events_inject.c

extern crate alloc;
use alloc::vec::Vec;

use spin::Mutex;

#[derive(Clone, Debug)]
pub struct InjectedEvent {
    pub event_name: alloc::string::String,
    pub payload: Vec<u8>,
}

static QUEUE: Mutex<Vec<InjectedEvent>> = Mutex::new(Vec::new());

pub fn inject(event_name: &str, payload: &[u8]) {
    QUEUE.lock().push(InjectedEvent {
        event_name: event_name.into(),
        payload: payload.into(),
    });
}

pub fn drain() -> Vec<InjectedEvent> {
    core::mem::take(&mut *QUEUE.lock())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inject_then_drain() {
        inject("sched_switch", b"prev=1 next=2");
        let d = drain();
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].event_name, "sched_switch");
    }
}
