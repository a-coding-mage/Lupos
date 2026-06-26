//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_dynevent.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_dynevent.c
//! Dynamic-event creation framework (`dynamic_events` file).
//!
//! Userspace can `write(2)` a definition like `p:myprobe sys_open` to register
//! a new kprobe-event or synthetic event at runtime.
//!
//! Ref: vendor/linux/kernel/trace/trace_dynevent.c

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

use spin::Mutex;

#[derive(Clone, Debug)]
pub struct DynEvent {
    pub name: String,
    pub kind: char,
    pub args: String,
}

static EVENTS: Mutex<Vec<DynEvent>> = Mutex::new(Vec::new());

/// Parse a definition line.  Accepts the legacy format `<kind>:<name> <args>`.
pub fn parse(line: &str) -> Result<DynEvent, i32> {
    let mut it = line.splitn(2, ' ');
    let header = it.next().ok_or(-22)?;
    let args = it.next().unwrap_or("").into();
    let mut h = header.splitn(2, ':');
    let kind = h.next().and_then(|s| s.chars().next()).ok_or(-22)?;
    let name = h.next().ok_or(-22)?.into();
    Ok(DynEvent { name, kind, args })
}

pub fn add(e: DynEvent) -> Result<(), i32> {
    let mut g = EVENTS.lock();
    if g.iter().any(|x| x.name == e.name) {
        return Err(-17);
    }
    g.push(e);
    Ok(())
}

pub fn remove(name: &str) -> Result<(), i32> {
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
    fn parse_kprobe_line() {
        let e = parse("p:myp sys_open").unwrap();
        assert_eq!(e.kind, 'p');
        assert_eq!(e.name, "myp");
        assert_eq!(e.args, "sys_open");
    }

    #[test]
    fn duplicate_add_is_eexist() {
        let e = parse("p:dupev sys_open").unwrap();
        add(e.clone()).unwrap();
        assert_eq!(add(e).unwrap_err(), -17);
        remove("dupev").unwrap();
    }
}
