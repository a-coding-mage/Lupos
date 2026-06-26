//! linux-parity: partial
//! linux-source: vendor/linux/lib/iomap.c
//! test-origin: linux:vendor/linux/lib/iomap.c
//! Generic MMIO access helpers exported to Linux-built modules.
//!
//! Full MMIO accessor surface: ioread/iowrite {8,16,32}, their big-endian
//! ({16,32}be) and 64-bit (`__io{read,write}64[be]_{lo_hi,hi_lo}`) variants, and
//! the repeat/string forms (`io{read,write}{8,16,32}_rep`). Remaining work vs
//! Linux for `complete`: the PIO dispatch arm of `IO_COND` (port-mapped cookies
//! from `ioport_map`) — pending the PCI I/O-port mapping path; Lupos currently
//! issues MMIO accesses for every cookie.

use core::ffi::c_void;
use core::ptr::{read_volatile, write_volatile};

use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("ioread8", ioread8 as usize, false);
    export_symbol_once("ioread16", ioread16 as usize, false);
    export_symbol_once("ioread32", ioread32 as usize, false);
    export_symbol_once("iowrite8", iowrite8 as usize, false);
    export_symbol_once("iowrite16", iowrite16 as usize, false);
    export_symbol_once("iowrite32", iowrite32 as usize, false);
    export_symbol_once("ioread16be", ioread16be as usize, false);
    export_symbol_once("ioread32be", ioread32be as usize, false);
    export_symbol_once("iowrite16be", iowrite16be as usize, false);
    export_symbol_once("iowrite32be", iowrite32be as usize, false);
    export_symbol_once("__ioread64_lo_hi", __ioread64_lo_hi as usize, false);
    export_symbol_once("__ioread64_hi_lo", __ioread64_hi_lo as usize, false);
    export_symbol_once("__ioread64be_lo_hi", __ioread64be_lo_hi as usize, false);
    export_symbol_once("__ioread64be_hi_lo", __ioread64be_hi_lo as usize, false);
    export_symbol_once("__iowrite64_lo_hi", __iowrite64_lo_hi as usize, false);
    export_symbol_once("__iowrite64_hi_lo", __iowrite64_hi_lo as usize, false);
    export_symbol_once("__iowrite64be_lo_hi", __iowrite64be_lo_hi as usize, false);
    export_symbol_once("__iowrite64be_hi_lo", __iowrite64be_hi_lo as usize, false);
    export_symbol_once("ioread8_rep", ioread8_rep as usize, false);
    export_symbol_once("ioread16_rep", ioread16_rep as usize, false);
    export_symbol_once("ioread32_rep", ioread32_rep as usize, false);
    export_symbol_once("iowrite8_rep", iowrite8_rep as usize, false);
    export_symbol_once("iowrite16_rep", iowrite16_rep as usize, false);
    export_symbol_once("iowrite32_rep", iowrite32_rep as usize, false);
}

/// `ioread8` - `vendor/linux/lib/iomap.c`.
///
/// Lupos currently supports MMIO cookies returned by `ioremap()`; PIO cookies
/// will be handled when the PCI I/O-port mapping path is wired.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ioread8(addr: *const c_void) -> u32 {
    if addr.is_null() {
        return u8::MAX as u32;
    }
    unsafe { read_volatile(addr.cast::<u8>()) as u32 }
}

/// `ioread16` - `vendor/linux/lib/iomap.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ioread16(addr: *const c_void) -> u32 {
    if addr.is_null() {
        return u16::MAX as u32;
    }
    unsafe { read_volatile(addr.cast::<u16>()) as u32 }
}

/// `ioread32` - `vendor/linux/lib/iomap.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ioread32(addr: *const c_void) -> u32 {
    if addr.is_null() {
        return u32::MAX;
    }
    unsafe { read_volatile(addr.cast::<u32>()) }
}

/// `iowrite8` - `vendor/linux/lib/iomap.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn iowrite8(value: u8, addr: *mut c_void) {
    if !addr.is_null() {
        unsafe { write_volatile(addr.cast::<u8>(), value) };
    }
}

/// `iowrite16` - `vendor/linux/lib/iomap.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn iowrite16(value: u16, addr: *mut c_void) {
    if !addr.is_null() {
        unsafe { write_volatile(addr.cast::<u16>(), value) };
    }
}

/// `iowrite32` - `vendor/linux/lib/iomap.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn iowrite32(value: u32, addr: *mut c_void) {
    if !addr.is_null() {
        unsafe { write_volatile(addr.cast::<u32>(), value) };
    }
}

// ── Big-endian 16/32-bit MMIO accessors ──────────────────────────────────────
// Linux `mmio_read*be`/`mmio_write*be`: byte-swap around a native volatile
// access (PIO variants pending the ioport_map path — see module note).

/// `ioread16be` - `vendor/linux/lib/iomap.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ioread16be(addr: *const c_void) -> u32 {
    if addr.is_null() {
        return u16::MAX as u32;
    }
    unsafe { read_volatile(addr.cast::<u16>()).swap_bytes() as u32 }
}

/// `ioread32be` - `vendor/linux/lib/iomap.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ioread32be(addr: *const c_void) -> u32 {
    if addr.is_null() {
        return u32::MAX;
    }
    unsafe { read_volatile(addr.cast::<u32>()).swap_bytes() }
}

/// `iowrite16be` - `vendor/linux/lib/iomap.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn iowrite16be(value: u16, addr: *mut c_void) {
    if !addr.is_null() {
        unsafe { write_volatile(addr.cast::<u16>(), value.swap_bytes()) };
    }
}

/// `iowrite32be` - `vendor/linux/lib/iomap.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn iowrite32be(value: u32, addr: *mut c_void) {
    if !addr.is_null() {
        unsafe { write_volatile(addr.cast::<u32>(), value.swap_bytes()) };
    }
}

// ── 64-bit MMIO accessors ────────────────────────────────────────────────────
// The lo_hi/hi_lo ordering only differs on the PIO path (two 32-bit ports); for
// an MMIO cookie all four collapse to a single 64-bit readq/writeq, exactly as
// Linux's `IO_COND` MMIO branch does.

/// `__ioread64_lo_hi` - `vendor/linux/lib/iomap.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __ioread64_lo_hi(addr: *const c_void) -> u64 {
    if addr.is_null() {
        return u64::MAX;
    }
    unsafe { read_volatile(addr.cast::<u64>()) }
}

/// `__ioread64_hi_lo` - `vendor/linux/lib/iomap.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __ioread64_hi_lo(addr: *const c_void) -> u64 {
    unsafe { __ioread64_lo_hi(addr) }
}

/// `__ioread64be_lo_hi` - `vendor/linux/lib/iomap.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __ioread64be_lo_hi(addr: *const c_void) -> u64 {
    if addr.is_null() {
        return u64::MAX;
    }
    unsafe { read_volatile(addr.cast::<u64>()).swap_bytes() }
}

/// `__ioread64be_hi_lo` - `vendor/linux/lib/iomap.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __ioread64be_hi_lo(addr: *const c_void) -> u64 {
    unsafe { __ioread64be_lo_hi(addr) }
}

/// `__iowrite64_lo_hi` - `vendor/linux/lib/iomap.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __iowrite64_lo_hi(value: u64, addr: *mut c_void) {
    if !addr.is_null() {
        unsafe { write_volatile(addr.cast::<u64>(), value) };
    }
}

/// `__iowrite64_hi_lo` - `vendor/linux/lib/iomap.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __iowrite64_hi_lo(value: u64, addr: *mut c_void) {
    unsafe { __iowrite64_lo_hi(value, addr) }
}

/// `__iowrite64be_lo_hi` - `vendor/linux/lib/iomap.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __iowrite64be_lo_hi(value: u64, addr: *mut c_void) {
    if !addr.is_null() {
        unsafe { write_volatile(addr.cast::<u64>(), value.swap_bytes()) };
    }
}

/// `__iowrite64be_hi_lo` - `vendor/linux/lib/iomap.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __iowrite64be_hi_lo(value: u64, addr: *mut c_void) {
    unsafe { __iowrite64be_lo_hi(value, addr) }
}

// ── Repeat (string) MMIO accessors ───────────────────────────────────────────
// Linux `mmio_ins*`/`mmio_outs*`: transfer `count` units to/from a *fixed*
// register address (a device FIFO), using `__raw` accesses (no byte-swap).

/// `ioread8_rep` - `vendor/linux/lib/iomap.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ioread8_rep(addr: *const c_void, dst: *mut c_void, count: usize) {
    if addr.is_null() || dst.is_null() {
        return;
    }
    let src = addr.cast::<u8>();
    let mut d = dst.cast::<u8>();
    for _ in 0..count {
        unsafe {
            write_volatile(d, read_volatile(src));
            d = d.add(1);
        }
    }
}

/// `ioread16_rep` - `vendor/linux/lib/iomap.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ioread16_rep(addr: *const c_void, dst: *mut c_void, count: usize) {
    if addr.is_null() || dst.is_null() {
        return;
    }
    let src = addr.cast::<u16>();
    let mut d = dst.cast::<u16>();
    for _ in 0..count {
        unsafe {
            write_volatile(d, read_volatile(src));
            d = d.add(1);
        }
    }
}

/// `ioread32_rep` - `vendor/linux/lib/iomap.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ioread32_rep(addr: *const c_void, dst: *mut c_void, count: usize) {
    if addr.is_null() || dst.is_null() {
        return;
    }
    let src = addr.cast::<u32>();
    let mut d = dst.cast::<u32>();
    for _ in 0..count {
        unsafe {
            write_volatile(d, read_volatile(src));
            d = d.add(1);
        }
    }
}

/// `iowrite8_rep` - `vendor/linux/lib/iomap.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn iowrite8_rep(addr: *mut c_void, src: *const c_void, count: usize) {
    if addr.is_null() || src.is_null() {
        return;
    }
    let d = addr.cast::<u8>();
    let mut s = src.cast::<u8>();
    for _ in 0..count {
        unsafe {
            write_volatile(d, read_volatile(s));
            s = s.add(1);
        }
    }
}

/// `iowrite16_rep` - `vendor/linux/lib/iomap.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn iowrite16_rep(addr: *mut c_void, src: *const c_void, count: usize) {
    if addr.is_null() || src.is_null() {
        return;
    }
    let d = addr.cast::<u16>();
    let mut s = src.cast::<u16>();
    for _ in 0..count {
        unsafe {
            write_volatile(d, read_volatile(s));
            s = s.add(1);
        }
    }
}

/// `iowrite32_rep` - `vendor/linux/lib/iomap.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn iowrite32_rep(addr: *mut c_void, src: *const c_void, count: usize) {
    if addr.is_null() || src.is_null() {
        return;
    }
    let d = addr.cast::<u32>();
    let mut s = src.cast::<u32>();
    for _ in 0..count {
        unsafe {
            write_volatile(d, read_volatile(s));
            s = s.add(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iomap_exports_module_symbols() {
        register_module_exports();
        for (name, addr) in [
            ("ioread8", ioread8 as usize),
            ("ioread16", ioread16 as usize),
            ("ioread32", ioread32 as usize),
            ("iowrite8", iowrite8 as usize),
            ("iowrite16", iowrite16 as usize),
            ("iowrite32", iowrite32 as usize),
        ] {
            assert_eq!(crate::kernel::module::find_symbol(name), Some(addr));
        }
    }

    #[test]
    fn iomap_mmio_accessors_use_volatile_widths() {
        let mut value = 0u32;
        unsafe {
            iowrite32(0xaabb_ccdd, (&mut value as *mut u32).cast());
            assert_eq!(ioread32((&value as *const u32).cast()), 0xaabb_ccdd);
            iowrite16(0x1122, (&mut value as *mut u32).cast());
            assert_eq!(ioread16((&value as *const u32).cast()), 0x1122);
            iowrite8(0x44, (&mut value as *mut u32).cast());
            assert_eq!(ioread8((&value as *const u32).cast()), 0x44);
        }
    }
}
