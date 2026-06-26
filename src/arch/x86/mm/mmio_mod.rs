//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/mm/mmio-mod.c
//! test-origin: linux:vendor/linux/arch/x86/mm/mmio-mod.c
//! MMIO tracing module policy.
//!
//! Mirrors the disabled module/export surface from
//! `vendor/linux/arch/x86/mm/mmio-mod.c`. Runtime MMIO tracing depends on
//! KMMIO traps, so Lupos returns stable unsupported errors.

use crate::include::uapi::errno::ENODEV;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MmioTraceEvent {
    pub phys: u64,
    pub width: u8,
    pub write: bool,
}

pub const fn mmiotrace_enabled() -> bool {
    false
}

pub const fn mmiotrace_printk(_event: MmioTraceEvent) -> Result<(), i32> {
    Err(ENODEV)
}

pub const fn mmio_trace_init(enabled: bool) -> Result<(), i32> {
    if enabled { Ok(()) } else { Err(ENODEV) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mmiotrace_export_fails_closed() {
        assert_eq!(
            mmiotrace_printk(MmioTraceEvent {
                phys: 0xfec0_0000,
                width: 4,
                write: true
            }),
            Err(ENODEV)
        );
    }
}
