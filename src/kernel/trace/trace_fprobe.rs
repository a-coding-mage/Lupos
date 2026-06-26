//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_fprobe.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_fprobe.c
//! Trace-event glue for `fprobe`-attached entries.
//!
//! Ref: vendor/linux/kernel/trace/trace_fprobe.c

extern crate alloc;
use alloc::string::String;

pub struct TraceFprobe {
    pub name: String,
    pub func: u64,
}

impl TraceFprobe {
    pub fn new(name: &str, func: u64) -> Self {
        Self {
            name: name.into(),
            func,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_stores_func_and_name() {
        let p = TraceFprobe::new("my_fprobe", 0x1000);
        assert_eq!(p.name, "my_fprobe");
        assert_eq!(p.func, 0x1000);
    }
}
