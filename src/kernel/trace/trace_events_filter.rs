//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_events_filter.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_events_filter.c
//! Per-event predicate filter (`events/<x>/<y>/filter`).
//!
//! Ref: vendor/linux/kernel/trace/trace_events_filter.c

extern crate alloc;
use alloc::string::String;

/// Very small filter parser: accepts `field <op> value` for `==`, `!=`, `<`, `>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Filter {
    pub field: String,
    pub op: FilterOp,
    pub value: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterOp {
    Eq,
    Ne,
    Lt,
    Gt,
}

pub fn parse(s: &str) -> Result<Filter, i32> {
    for (op_str, op) in [
        ("==", FilterOp::Eq),
        ("!=", FilterOp::Ne),
        ("<", FilterOp::Lt),
        (">", FilterOp::Gt),
    ] {
        if let Some(idx) = s.find(op_str) {
            let field = s[..idx].trim().into();
            let value = s[idx + op_str.len()..].trim().parse().map_err(|_| -22)?;
            return Ok(Filter { field, op, value });
        }
    }
    Err(-22)
}

pub fn matches(f: &Filter, value: i64) -> bool {
    match f.op {
        FilterOp::Eq => value == f.value,
        FilterOp::Ne => value != f.value,
        FilterOp::Lt => value < f.value,
        FilterOp::Gt => value > f.value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_eq_filter() {
        let f = parse("pid == 100").unwrap();
        assert_eq!(f.field, "pid");
        assert_eq!(f.op, FilterOp::Eq);
        assert_eq!(f.value, 100);
        assert!(matches(&f, 100));
        assert!(!matches(&f, 99));
    }

    #[test]
    fn parse_lt() {
        let f = parse("cpu < 4").unwrap();
        assert!(matches(&f, 3));
        assert!(!matches(&f, 4));
    }

    #[test]
    fn invalid_returns_einval() {
        assert_eq!(parse("garbage").unwrap_err(), -22);
    }
}
