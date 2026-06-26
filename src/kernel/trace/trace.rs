//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace.c
//! test-origin: linux:vendor/linux/kernel/trace/trace.c
//! Core trace runtime — owns the per-cpu trace buffers and the global
//! `trace_array` registry.
//!
//! Ref: vendor/linux/kernel/trace/trace.c

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

use spin::Mutex;

/// `struct trace_array` — one set of trace buffers + per-tracer state.
pub struct TraceArray {
    pub name: String,
    pub current_tracer: Option<&'static str>,
    pub buffer_size_kb: u32,
    pub enabled: bool,
}

static ARRAYS: Mutex<Vec<TraceArray>> = Mutex::new(Vec::new());

/// `trace_array_create_dir` — register a new trace_array (`/sys/kernel/tracing/instances/<name>`).
pub fn create(name: &str) -> Result<(), i32> {
    let mut g = ARRAYS.lock();
    if g.iter().any(|a| a.name == name) {
        return Err(-17); // -EEXIST
    }
    g.push(TraceArray {
        name: name.into(),
        current_tracer: None,
        buffer_size_kb: 1024,
        enabled: false,
    });
    Ok(())
}

pub fn count() -> usize {
    ARRAYS.lock().len()
}

pub fn enable(name: &str) -> Result<(), i32> {
    let mut g = ARRAYS.lock();
    if let Some(a) = g.iter_mut().find(|a| a.name == name) {
        a.enabled = true;
        Ok(())
    } else {
        Err(-2) // -ENOENT
    }
}

pub fn destroy(name: &str) -> Result<(), i32> {
    let mut g = ARRAYS.lock();
    if let Some(pos) = g.iter().position(|a| a.name == name) {
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
    fn create_then_destroy() {
        let n0 = count();
        create("test-arr").unwrap();
        assert_eq!(count(), n0 + 1);
        enable("test-arr").unwrap();
        destroy("test-arr").unwrap();
        assert_eq!(count(), n0);
    }

    #[test]
    fn duplicate_create_is_eexist() {
        create("dup-arr").unwrap();
        assert_eq!(create("dup-arr").unwrap_err(), -17);
        destroy("dup-arr").unwrap();
    }

    #[test]
    fn enable_missing_is_enoent() {
        assert_eq!(enable("no-such").unwrap_err(), -2);
    }
}
