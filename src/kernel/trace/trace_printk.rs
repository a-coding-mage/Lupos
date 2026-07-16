//! linux-parity: partial
//! linux-source: vendor/linux/kernel/trace/trace_printk.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_printk.c
//! `trace_printk()` — printk-shaped emit into the trace ring buffer (not the
//! printk ring).  Useful for debugging without touching dmesg.
//!
//! Ref: vendor/linux/kernel/trace/trace_printk.c

extern crate alloc;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use spin::Mutex;

static SINK: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Permanent copies made by `hold_module_trace_bprintk_format()`.  Vendor
/// Linux intentionally does not remove these at module unload: trace buffers
/// may still contain binary printk records keyed by the old format address.
static MODULE_FORMATS: Mutex<Vec<Box<[u8]>>> = Mutex::new(Vec::new());

pub fn emit_str(s: &str) {
    SINK.lock().push(s.into());
}

pub fn drain() -> Vec<String> {
    core::mem::take(&mut *SINK.lock())
}

unsafe fn copy_c_string(address: usize) -> Result<Box<[u8]>, i32> {
    if address == 0 {
        return Err(-22); // EINVAL
    }
    const MAX_FORMAT: usize = 64 * 1024;
    let pointer = address as *const u8;
    let mut length = 0usize;
    while length < MAX_FORMAT {
        // SAFETY: the module loader has validated that relocated metadata
        // pointers target retained module memory before calling this hook.
        if unsafe { pointer.add(length).read() } == 0 {
            let bytes = unsafe { core::slice::from_raw_parts(pointer, length + 1) };
            return Ok(bytes.to_vec().into_boxed_slice());
        }
        length += 1;
    }
    Err(-8) // ENOEXEC: unterminated module metadata string
}

/// `hold_module_trace_bprintk_format()` at `MODULE_STATE_COMING`.
///
/// The relocated pointer array is rewritten to permanent, deduplicated copies
/// exactly because the original strings disappear with module memory.
///
/// # Safety
/// Every non-null relocated pointer in `section` must address a readable,
/// NUL-terminated string retained for the duration of this call.
pub unsafe fn module_coming(section: &mut [u8]) -> Result<(), i32> {
    if section.len() % core::mem::size_of::<usize>() != 0 {
        return Err(-8);
    }

    for pointer_bytes in section.chunks_exact_mut(core::mem::size_of::<usize>()) {
        let address = usize::from_le_bytes(pointer_bytes.try_into().map_err(|_| -8)?);
        let copied = unsafe { copy_c_string(address)? };

        let mut formats = MODULE_FORMATS.lock();
        let permanent = formats
            .iter()
            .find(|format| format.as_ref() == copied.as_ref())
            .map(|format| format.as_ptr() as usize)
            .unwrap_or_else(|| {
                let address = copied.as_ptr() as usize;
                formats.push(copied);
                address
            });
        pointer_bytes.copy_from_slice(&permanent.to_le_bytes());
    }
    Ok(())
}

pub fn module_format_addresses() -> Vec<usize> {
    MODULE_FORMATS
        .lock()
        .iter()
        .map(|format| format.as_ptr() as usize)
        .collect()
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use super::*;

    #[test]
    fn emit_then_drain() {
        emit_str("hello");
        emit_str("world");
        let d = drain();
        assert_eq!(d, alloc::vec!["hello".to_string(), "world".to_string()]);
    }
}
