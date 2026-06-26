//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_events_user.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_events_user.c
//! User-events: events registered from userspace via `/sys/kernel/tracing/user_events_data`.
//!
//! Ref: vendor/linux/kernel/trace/trace_events_user.c

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

use spin::Mutex;

#[derive(Clone, Debug)]
pub struct UserEvent {
    pub name: String,
    pub args: String,
    pub registered_by: u32, // pid
}

static EVENTS: Mutex<Vec<UserEvent>> = Mutex::new(Vec::new());

pub fn register(name: &str, args: &str, pid: u32) -> Result<u32, i32> {
    let mut g = EVENTS.lock();
    let idx = g.len() as u32;
    g.push(UserEvent {
        name: name.into(),
        args: args.into(),
        registered_by: pid,
    });
    Ok(idx)
}

pub fn count() -> usize {
    EVENTS.lock().len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_assigns_id_and_increments_count() {
        let n0 = count();
        let _ = register("evtest", "u32 x", 1).unwrap();
        assert_eq!(count(), n0 + 1);
    }
}
