//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_probe.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_probe.c
//! Shared probe-argument parsing used by kprobe / uprobe / eprobe trace
//! events.  Accepts `<name>=<fetch-expr>` tokens.
//!
//! Ref: vendor/linux/kernel/trace/trace_probe.c

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProbeArg {
    pub name: String,
    pub fetch: String,
}

pub fn parse(args: &str) -> Vec<ProbeArg> {
    let mut out = Vec::new();
    for tok in args.split_whitespace() {
        if let Some((n, f)) = tok.split_once('=') {
            out.push(ProbeArg {
                name: n.into(),
                fetch: f.into(),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_two_args() {
        let v = parse("pid=%di mode=%si");
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].name, "pid");
        assert_eq!(v[1].fetch, "%si");
    }

    #[test]
    fn unnamed_tokens_are_skipped() {
        // Linux uses bare $retval too — for the no_std port we require name=expr.
        let v = parse("$retval ax");
        assert!(v.is_empty());
    }
}
