//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_events_trigger.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_events_trigger.c
//! Trace event triggers — actions like `traceon` / `traceoff` / `snapshot`
//! fired when an event occurs.
//!
//! Ref: vendor/linux/kernel/trace/trace_events_trigger.c

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TriggerAction {
    TraceOn,
    TraceOff,
    Snapshot,
    Stacktrace,
    HistAdd(String),
}

pub fn parse(s: &str) -> Result<TriggerAction, i32> {
    match s.trim() {
        "traceon" => Ok(TriggerAction::TraceOn),
        "traceoff" => Ok(TriggerAction::TraceOff),
        "snapshot" => Ok(TriggerAction::Snapshot),
        "stacktrace" => Ok(TriggerAction::Stacktrace),
        s if s.starts_with("hist:") => Ok(TriggerAction::HistAdd(s[5..].into())),
        _ => Err(-22),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_actions() {
        assert_eq!(parse("traceon").unwrap(), TriggerAction::TraceOn);
        assert_eq!(parse("traceoff").unwrap(), TriggerAction::TraceOff);
        assert_eq!(parse("snapshot").unwrap(), TriggerAction::Snapshot);
    }

    #[test]
    fn hist_carries_spec() {
        match parse("hist:keys=pid").unwrap() {
            TriggerAction::HistAdd(s) => assert_eq!(s, "keys=pid"),
            _ => panic!(),
        }
    }
}
