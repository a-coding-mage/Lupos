//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_boot.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_boot.c
//! Early-boot tracing — parse the `ftrace.*=` cmdline so tracers can fire
//! before `tracefs` is mounted.
//!
//! Ref: vendor/linux/kernel/trace/trace_boot.c

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BootTracerCfg {
    pub name: String,
    pub options: Vec<String>,
}

pub fn parse(cmdline: &str) -> Vec<BootTracerCfg> {
    let mut out = Vec::new();
    for token in cmdline.split_whitespace() {
        if let Some(rest) = token.strip_prefix("ftrace=") {
            let mut parts = rest.split(',');
            let name: String = parts.next().unwrap_or("").into();
            let options: Vec<String> = parts.map(Into::into).collect();
            out.push(BootTracerCfg { name, options });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ftrace_kv_pair() {
        let r = parse("foo=bar ftrace=function,sym=do_sys_open");
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].name, "function");
        assert_eq!(r[0].options, ["sym=do_sys_open"]);
    }

    #[test]
    fn no_ftrace_returns_empty() {
        assert!(parse("nokaslr console=ttyS0").is_empty());
    }
}
