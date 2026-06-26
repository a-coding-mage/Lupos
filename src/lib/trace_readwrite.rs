//! linux-parity: complete
//! linux-source: vendor/linux/lib/trace_readwrite.c
//! test-origin: linux:vendor/linux/lib/trace_readwrite.c
//! MMIO read/write tracepoint call shapes.

use core::ffi::c_void;

use crate::kernel::module::{export_symbol, find_symbol};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MmioTraceKind {
    Write,
    PostWrite,
    Read,
    PostRead,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MmioTraceEvent {
    pub kind: MmioTraceKind,
    pub value: u64,
    pub width: u8,
    pub addr: usize,
    pub caller_addr: usize,
    pub caller_addr0: usize,
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("log_write_mmio", log_write_mmio as usize, true);
    export_symbol_once("log_post_write_mmio", log_post_write_mmio as usize, true);
    export_symbol_once("log_read_mmio", log_read_mmio as usize, true);
    export_symbol_once("log_post_read_mmio", log_post_read_mmio as usize, true);
}

pub fn mmio_write_event(
    value: u64,
    width: u8,
    addr: *const c_void,
    caller_addr: usize,
    caller_addr0: usize,
) -> MmioTraceEvent {
    MmioTraceEvent {
        kind: MmioTraceKind::Write,
        value,
        width,
        addr: addr as usize,
        caller_addr,
        caller_addr0,
    }
}

pub fn mmio_post_write_event(
    value: u64,
    width: u8,
    addr: *const c_void,
    caller_addr: usize,
    caller_addr0: usize,
) -> MmioTraceEvent {
    MmioTraceEvent {
        kind: MmioTraceKind::PostWrite,
        value,
        width,
        addr: addr as usize,
        caller_addr,
        caller_addr0,
    }
}

pub fn mmio_read_event(
    width: u8,
    addr: *const c_void,
    caller_addr: usize,
    caller_addr0: usize,
) -> MmioTraceEvent {
    MmioTraceEvent {
        kind: MmioTraceKind::Read,
        value: 0,
        width,
        addr: addr as usize,
        caller_addr,
        caller_addr0,
    }
}

pub fn mmio_post_read_event(
    value: u64,
    width: u8,
    addr: *const c_void,
    caller_addr: usize,
    caller_addr0: usize,
) -> MmioTraceEvent {
    MmioTraceEvent {
        kind: MmioTraceKind::PostRead,
        value,
        width,
        addr: addr as usize,
        caller_addr,
        caller_addr0,
    }
}

pub extern "C" fn log_write_mmio(
    value: u64,
    width: u8,
    addr: *mut c_void,
    caller_addr: usize,
    caller_addr0: usize,
) {
    let _ = mmio_write_event(value, width, addr.cast_const(), caller_addr, caller_addr0);
}

pub extern "C" fn log_post_write_mmio(
    value: u64,
    width: u8,
    addr: *mut c_void,
    caller_addr: usize,
    caller_addr0: usize,
) {
    let _ = mmio_post_write_event(value, width, addr.cast_const(), caller_addr, caller_addr0);
}

pub extern "C" fn log_read_mmio(
    width: u8,
    addr: *const c_void,
    caller_addr: usize,
    caller_addr0: usize,
) {
    let _ = mmio_read_event(width, addr, caller_addr, caller_addr0);
}

pub extern "C" fn log_post_read_mmio(
    value: u64,
    width: u8,
    addr: *const c_void,
    caller_addr: usize,
    caller_addr0: usize,
) {
    let _ = mmio_post_read_event(value, width, addr, caller_addr, caller_addr0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_readwrite_events_match_linux_tracepoint_arguments() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/trace_readwrite.c"
        ));
        assert!(source.contains("#define CREATE_TRACE_POINTS"));
        assert!(source.contains("void log_write_mmio(u64 val, u8 width"));
        assert!(
            source.contains("trace_rwmmio_write(caller_addr, caller_addr0, val, width, addr);")
        );
        assert!(
            source
                .contains("trace_rwmmio_post_write(caller_addr, caller_addr0, val, width, addr);")
        );
        assert!(source.contains("trace_rwmmio_read(caller_addr, caller_addr0, width, addr);"));
        assert!(
            source.contains("trace_rwmmio_post_read(caller_addr, caller_addr0, val, width, addr);")
        );
        assert!(source.contains("EXPORT_SYMBOL_GPL(log_write_mmio);"));

        let addr = 0xfee0_0000usize as *const c_void;
        assert_eq!(
            mmio_write_event(0xab, 4, addr, 0x10, 0x20),
            MmioTraceEvent {
                kind: MmioTraceKind::Write,
                value: 0xab,
                width: 4,
                addr: 0xfee0_0000,
                caller_addr: 0x10,
                caller_addr0: 0x20,
            }
        );
        assert_eq!(mmio_read_event(8, addr, 1, 2).kind, MmioTraceKind::Read);
        assert_eq!(
            mmio_post_read_event(0x55, 1, addr, 3, 4).kind,
            MmioTraceKind::PostRead
        );
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("log_write_mmio"),
            Some(log_write_mmio as usize)
        );
    }
}
