//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/io.h
//! test-origin: linux:vendor/linux/arch/x86/boot/io.h
//! Indirect port-I/O seam for the real-mode setup stub.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/io.h
//! - vendor/linux/arch/x86/include/asm/shared/io.h (the `__inb/__outb/__outw`
//!   primitives)
//!
//! Linux routes the setup code's `inb`/`outb`/`outw` through a global
//! `struct port_io_ops pio_ops` so that confidential-compute guests can
//! swap in alternate transports. `init_default_io_ops()` wires the
//! callbacks to the normal `in`/`out` instructions; a TDX guest overrides
//! them with hypercall-based helpers. The `#define inb pio_ops.f_inb`
//! macros make every caller dispatch through the vtable.
//!
//! We model this faithfully:
//!   * [`raw_inb`]/[`raw_outb`]/[`raw_outw`] are the genuine `in`/`out`
//!     instruction wrappers (Linux `__inb`/`__outb`/`__outw`).
//!   * [`PortIoOps`] holds the three callbacks (`f_inb`/`f_outb`/`f_outw`).
//!   * [`init_default_io_ops`] populates them with the raw instructions.
//!   * [`PortIoOps::inb`]/[`outb`]/[`outw`] redirect through the callbacks,
//!     exactly like the `#define`s — so a TDX-style override is a drop-in.

/// Read a byte from I/O port `port` (`inb`).
///
/// Linux `__inb` (asm/shared/io.h): `inb %dx, %al`. This is a real,
/// complete port read; the unsafe block is the irreducible hardware
/// access and the public wrapper is safe to call from setup code that
/// owns the port.
#[inline]
pub fn raw_inb(port: u16) -> u8 {
    let val: u8;
    // SAFETY: a plain `inb` has no memory side effects; the caller is the
    // boot stub which owns legacy port space. Mirrors `__inb` in
    // vendor/linux/arch/x86/include/asm/shared/io.h.
    unsafe {
        core::arch::asm!(
            "in al, dx",
            in("dx") port,
            out("al") val,
            options(nomem, nostack, preserves_flags),
        );
    }
    val
}

/// Write a byte `v` to I/O port `port` (`outb`).
///
/// Linux `__outb`: `outb %al, %dx`.
#[inline]
pub fn raw_outb(v: u8, port: u16) {
    // SAFETY: a plain `outb` has no memory side effects; the caller owns
    // the port. Mirrors `__outb` in asm/shared/io.h.
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") port,
            in("al") v,
            options(nomem, nostack, preserves_flags),
        );
    }
}

/// Write a word `v` to I/O port `port` (`outw`).
///
/// Linux `__outw`: `outw %ax, %dx`.
#[inline]
pub fn raw_outw(v: u16, port: u16) {
    // SAFETY: a plain `outw` has no memory side effects; the caller owns
    // the port. Mirrors `__outw` in asm/shared/io.h.
    unsafe {
        core::arch::asm!(
            "out dx, ax",
            in("dx") port,
            in("ax") v,
            options(nomem, nostack, preserves_flags),
        );
    }
}

/// Linux `struct port_io_ops` — the indirection vtable.
///
/// Holds the three function pointers the setup code dispatches through.
/// TDX guests overwrite these with hypercall helpers; everyone else uses
/// the raw `in`/`out` instructions installed by [`init_default_io_ops`].
#[derive(Clone, Copy)]
pub struct PortIoOps {
    /// `u8 (*f_inb)(u16 port)`
    pub f_inb: fn(u16) -> u8,
    /// `void (*f_outb)(u8 v, u16 port)`
    pub f_outb: fn(u8, u16),
    /// `void (*f_outw)(u16 v, u16 port)`
    pub f_outw: fn(u16, u16),
}

impl PortIoOps {
    /// `init_default_io_ops()` — wire the callbacks to the normal I/O
    /// instructions. Mirrors io.h lines 26-31:
    /// `f_inb = __inb; f_outb = __outb; f_outw = __outw;`.
    #[inline]
    pub const fn default_ops() -> Self {
        PortIoOps {
            f_inb: raw_inb,
            f_outb: raw_outb,
            f_outw: raw_outw,
        }
    }

    /// `inb` — redirected port read (`#define inb pio_ops.f_inb`).
    #[inline]
    pub fn inb(&self, port: u16) -> u8 {
        (self.f_inb)(port)
    }

    /// `outb` — redirected port write (`#define outb pio_ops.f_outb`).
    #[inline]
    pub fn outb(&self, v: u8, port: u16) {
        (self.f_outb)(v, port)
    }

    /// `outw` — redirected word write (`#define outw pio_ops.f_outw`).
    #[inline]
    pub fn outw(&self, v: u16, port: u16) {
        (self.f_outw)(v, port)
    }
}

/// `init_default_io_ops()` — construct the default `pio_ops`.
///
/// Linux mutates the global `pio_ops` in place; Rust prefers returning the
/// initialized value, which the boot driver stores in its own `pio_ops`
/// slot. The result is identical: the three callbacks point at the raw
/// `in`/`out` instructions.
#[inline]
pub const fn init_default_io_ops() -> PortIoOps {
    PortIoOps::default_ops()
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicU32, Ordering};

    // The raw_* asm wrappers touch real hardware ports, so the unit tests
    // exercise the *indirection* (which is the ABI-sensitive part) using
    // recording callbacks instead. This proves dispatch goes through the
    // vtable exactly like Linux's `#define inb pio_ops.f_inb`. The tests run
    // single-threaded (`--test-threads=1`); the atomics keep them sound
    // without needing `std` in this `no_std` crate.
    static LAST_INB_PORT: AtomicU32 = AtomicU32::new(0);
    static LAST_OUTB: AtomicU32 = AtomicU32::new(0); // (val << 16) | port
    static LAST_OUTW: AtomicU32 = AtomicU32::new(0); // (val << 16) | port

    fn fake_inb(port: u16) -> u8 {
        LAST_INB_PORT.store(port as u32, Ordering::Relaxed);
        0x5a
    }
    fn fake_outb(v: u8, port: u16) {
        LAST_OUTB.store(((v as u32) << 16) | port as u32, Ordering::Relaxed);
    }
    fn fake_outw(v: u16, port: u16) {
        LAST_OUTW.store(((v as u32) << 16) | port as u32, Ordering::Relaxed);
    }

    fn custom_ops() -> PortIoOps {
        PortIoOps {
            f_inb: fake_inb,
            f_outb: fake_outb,
            f_outw: fake_outw,
        }
    }

    #[test]
    fn default_ops_wire_to_raw_instructions() {
        let ops = init_default_io_ops();
        // Function-pointer identity: the defaults are the raw wrappers,
        // matching `f_inb = __inb` etc.
        assert_eq!(ops.f_inb as usize, raw_inb as usize);
        assert_eq!(ops.f_outb as usize, raw_outb as usize);
        assert_eq!(ops.f_outw as usize, raw_outw as usize);
    }

    #[test]
    fn inb_dispatches_through_callback() {
        let ops = custom_ops();
        let v = ops.inb(0x3f8);
        assert_eq!(v, 0x5a);
        assert_eq!(LAST_INB_PORT.load(Ordering::Relaxed), 0x3f8);
    }

    #[test]
    fn outb_dispatches_through_callback() {
        let ops = custom_ops();
        ops.outb(0x42, 0x80);
        assert_eq!(LAST_OUTB.load(Ordering::Relaxed), (0x42 << 16) | 0x80);
    }

    #[test]
    fn outw_dispatches_through_callback() {
        let ops = custom_ops();
        ops.outw(0xdead, 0xcf8);
        assert_eq!(LAST_OUTW.load(Ordering::Relaxed), (0xdead << 16) | 0xcf8);
    }
}
