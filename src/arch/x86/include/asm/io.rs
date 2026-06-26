//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/include/asm/io.h
//! test-origin: linux:vendor/linux/arch/x86/include/asm/io.h
//! x86 port-I/O and MMIO accessor helpers.
//!
//! Mirrors Linux `arch/x86/include/asm/io.h` for the pieces this crate owns:
//! scalar port I/O, REP string port I/O, pausing port I/O, MMIO
//! read/write/raw/relaxed aliases, and the `memcpy_fromio`,
//! `memcpy_toio`, and `memset_io` declarations routed to the x86 iomem
//! implementation. Mapping helpers such as `ioremap`, `iounmap`,
//! `virt_to_phys`, and `ioread/iowrite` intentionally live in their
//! Linux-shaped modules.

/// Write a single byte to an x86 I/O port.
///
/// # Safety
/// The caller must ensure that writing to `port` is valid for the current
/// hardware configuration.
#[inline(always)]
pub unsafe fn outb(port: u16, val: u8) {
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") port,
            in("al") val,
            options(nomem, nostack, preserves_flags),
        );
    }
}

/// Read a single byte from an x86 I/O port.
///
/// # Safety
/// The caller must ensure that reading from `port` is valid.
#[inline(always)]
pub unsafe fn inb(port: u16) -> u8 {
    let val: u8;
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

/// Read a 16-bit word from an x86 I/O port.
///
/// # Safety
/// See [`inb`].
#[inline(always)]
pub unsafe fn inw(port: u16) -> u16 {
    let val: u16;
    unsafe {
        core::arch::asm!(
            "in ax, dx",
            in("dx") port,
            out("ax") val,
            options(nomem, nostack, preserves_flags),
        );
    }
    val
}

/// Read a 32-bit dword from an x86 I/O port.
///
/// # Safety
/// See [`inb`].
#[inline(always)]
pub unsafe fn inl(port: u16) -> u32 {
    let val: u32;
    unsafe {
        core::arch::asm!(
            "in eax, dx",
            in("dx") port,
            out("eax") val,
            options(nomem, nostack, preserves_flags),
        );
    }
    val
}

/// Write a 16-bit word to an x86 I/O port.
///
/// # Safety
/// See [`outb`].
#[inline(always)]
pub unsafe fn outw(port: u16, val: u16) {
    unsafe {
        core::arch::asm!(
            "out dx, ax",
            in("dx") port,
            in("ax") val,
            options(nomem, nostack, preserves_flags),
        );
    }
}

/// Write a 32-bit dword to an x86 I/O port.
///
/// # Safety
/// See [`outb`].
#[inline(always)]
pub unsafe fn outl(port: u16, val: u32) {
    unsafe {
        core::arch::asm!(
            "out dx, eax",
            in("dx") port,
            in("eax") val,
            options(nomem, nostack, preserves_flags),
        );
    }
}

/// Linux `native_io_delay`: delay via the conventional POST port.
///
/// # Safety
/// Performs x86 port I/O.
#[inline(always)]
pub unsafe fn native_io_delay() {
    unsafe {
        outb(0x80, 0);
    }
}

/// Linux `slow_down_io`.
///
/// # Safety
/// Performs x86 port I/O.
#[inline(always)]
pub unsafe fn slow_down_io() {
    unsafe {
        native_io_delay();
    }
}

/// Pausing byte output, Linux `outb_p`.
///
/// # Safety
/// See [`outb`].
#[inline(always)]
pub unsafe fn outb_p(port: u16, val: u8) {
    unsafe {
        outb(port, val);
        slow_down_io();
    }
}

/// Pausing word output, Linux `outw_p`.
///
/// # Safety
/// See [`outw`].
#[inline(always)]
pub unsafe fn outw_p(port: u16, val: u16) {
    unsafe {
        outw(port, val);
        slow_down_io();
    }
}

/// Pausing dword output, Linux `outl_p`.
///
/// # Safety
/// See [`outl`].
#[inline(always)]
pub unsafe fn outl_p(port: u16, val: u32) {
    unsafe {
        outl(port, val);
        slow_down_io();
    }
}

/// Pausing byte input, Linux `inb_p`.
///
/// # Safety
/// See [`inb`].
#[inline(always)]
pub unsafe fn inb_p(port: u16) -> u8 {
    let val = unsafe { inb(port) };
    unsafe {
        slow_down_io();
    }
    val
}

/// Pausing word input, Linux `inw_p`.
///
/// # Safety
/// See [`inw`].
#[inline(always)]
pub unsafe fn inw_p(port: u16) -> u16 {
    let val = unsafe { inw(port) };
    unsafe {
        slow_down_io();
    }
    val
}

/// Pausing dword input, Linux `inl_p`.
///
/// # Safety
/// See [`inl`].
#[inline(always)]
pub unsafe fn inl_p(port: u16) -> u32 {
    let val = unsafe { inl(port) };
    unsafe {
        slow_down_io();
    }
    val
}

/// Read `count` bytes from `port` into `dst`, Linux `insb`.
///
/// # Safety
/// `dst` must cover `count` writable bytes and the port must be valid.
#[inline]
pub unsafe fn insb(port: u16, dst: *mut u8, count: usize) {
    unsafe {
        core::arch::asm!(
            "rep insb",
            in("dx") port,
            inout("rdi") dst => _,
            inout("rcx") count => _,
            options(nostack, preserves_flags),
        );
    }
}

/// Read `count` words from `port` into `dst`, Linux `insw`.
///
/// # Safety
/// `dst` must cover `count` writable words and the port must be valid.
#[inline]
pub unsafe fn insw(port: u16, dst: *mut u16, count: usize) {
    unsafe {
        core::arch::asm!(
            "rep insw",
            in("dx") port,
            inout("rdi") dst => _,
            inout("rcx") count => _,
            options(nostack, preserves_flags),
        );
    }
}

/// Read `count` dwords from `port` into `dst`, Linux `insl`.
///
/// # Safety
/// `dst` must cover `count` writable dwords and the port must be valid.
#[inline]
pub unsafe fn insl(port: u16, dst: *mut u32, count: usize) {
    unsafe {
        core::arch::asm!(
            "rep insd",
            in("dx") port,
            inout("rdi") dst => _,
            inout("rcx") count => _,
            options(nostack, preserves_flags),
        );
    }
}

/// Write `count` bytes from `src` to `port`, Linux `outsb`.
///
/// # Safety
/// `src` must cover `count` readable bytes and the port must be valid.
#[inline]
pub unsafe fn outsb(port: u16, src: *const u8, count: usize) {
    unsafe {
        core::arch::asm!(
            "rep outsb",
            in("dx") port,
            inout("rsi") src => _,
            inout("rcx") count => _,
            options(nostack, preserves_flags, readonly),
        );
    }
}

/// Write `count` words from `src` to `port`, Linux `outsw`.
///
/// # Safety
/// `src` must cover `count` readable words and the port must be valid.
#[inline]
pub unsafe fn outsw(port: u16, src: *const u16, count: usize) {
    unsafe {
        core::arch::asm!(
            "rep outsw",
            in("dx") port,
            inout("rsi") src => _,
            inout("rcx") count => _,
            options(nostack, preserves_flags, readonly),
        );
    }
}

/// Write `count` dwords from `src` to `port`, Linux `outsl`.
///
/// # Safety
/// `src` must cover `count` readable dwords and the port must be valid.
#[inline]
pub unsafe fn outsl(port: u16, src: *const u32, count: usize) {
    unsafe {
        core::arch::asm!(
            "rep outsd",
            in("dx") port,
            inout("rsi") src => _,
            inout("rcx") count => _,
            options(nostack, preserves_flags, readonly),
        );
    }
}

/// Pausing byte string input, Linux `insb_p`.
///
/// # Safety
/// See [`insb`].
#[inline]
pub unsafe fn insb_p(port: u16, dst: *mut u8, count: usize) {
    unsafe {
        insb(port, dst, count);
        slow_down_io();
    }
}

/// Pausing word string input, Linux `insw_p`.
///
/// # Safety
/// See [`insw`].
#[inline]
pub unsafe fn insw_p(port: u16, dst: *mut u16, count: usize) {
    unsafe {
        insw(port, dst, count);
        slow_down_io();
    }
}

/// Pausing dword string input, Linux `insl_p`.
///
/// # Safety
/// See [`insl`].
#[inline]
pub unsafe fn insl_p(port: u16, dst: *mut u32, count: usize) {
    unsafe {
        insl(port, dst, count);
        slow_down_io();
    }
}

/// Pausing byte string output, Linux `outsb_p`.
///
/// # Safety
/// See [`outsb`].
#[inline]
pub unsafe fn outsb_p(port: u16, src: *const u8, count: usize) {
    unsafe {
        outsb(port, src, count);
        slow_down_io();
    }
}

/// Pausing word string output, Linux `outsw_p`.
///
/// # Safety
/// See [`outsw`].
#[inline]
pub unsafe fn outsw_p(port: u16, src: *const u16, count: usize) {
    unsafe {
        outsw(port, src, count);
        slow_down_io();
    }
}

/// Pausing dword string output, Linux `outsl_p`.
///
/// # Safety
/// See [`outsl`].
#[inline]
pub unsafe fn outsl_p(port: u16, src: *const u32, count: usize) {
    unsafe {
        outsl(port, src, count);
        slow_down_io();
    }
}

/// `readb` - read a byte from MMIO.
///
/// # Safety
/// `addr` must be a valid MMIO mapping.
#[inline(always)]
pub unsafe fn readb(addr: *const u8) -> u8 {
    unsafe { core::ptr::read_volatile(addr) }
}

/// `readw` - read a word from MMIO.
///
/// # Safety
/// See [`readb`].
#[inline(always)]
pub unsafe fn readw(addr: *const u16) -> u16 {
    unsafe { core::ptr::read_volatile(addr) }
}

/// `readl` - read a dword from MMIO.
///
/// # Safety
/// See [`readb`].
#[inline(always)]
pub unsafe fn readl(addr: *const u32) -> u32 {
    unsafe { core::ptr::read_volatile(addr) }
}

/// `readq` - read a qword from MMIO.
///
/// # Safety
/// See [`readb`].
#[inline(always)]
pub unsafe fn readq(addr: *const u64) -> u64 {
    unsafe { core::ptr::read_volatile(addr) }
}

/// `writeb` - write a byte to MMIO.
///
/// # Safety
/// `addr` must be a valid MMIO mapping.
#[inline(always)]
pub unsafe fn writeb(val: u8, addr: *mut u8) {
    unsafe { core::ptr::write_volatile(addr, val) }
}

/// `writew` - write a word to MMIO.
///
/// # Safety
/// See [`writeb`].
#[inline(always)]
pub unsafe fn writew(val: u16, addr: *mut u16) {
    unsafe { core::ptr::write_volatile(addr, val) }
}

/// `writel` - write a dword to MMIO.
///
/// # Safety
/// See [`writeb`].
#[inline(always)]
pub unsafe fn writel(val: u32, addr: *mut u32) {
    unsafe { core::ptr::write_volatile(addr, val) }
}

/// `writeq` - write a qword to MMIO.
///
/// # Safety
/// See [`writeb`].
#[inline(always)]
pub unsafe fn writeq(val: u64, addr: *mut u64) {
    unsafe { core::ptr::write_volatile(addr, val) }
}

/// Linux `__readb`, also exported as `__raw_readb`.
///
/// # Safety
/// See [`readb`].
#[inline(always)]
pub unsafe fn __readb(addr: *const u8) -> u8 {
    unsafe { core::ptr::read_volatile(addr) }
}

/// Linux `__readw`, also exported as `__raw_readw`.
///
/// # Safety
/// See [`readb`].
#[inline(always)]
pub unsafe fn __readw(addr: *const u16) -> u16 {
    unsafe { core::ptr::read_volatile(addr) }
}

/// Linux `__readl`, also exported as `__raw_readl`.
///
/// # Safety
/// See [`readb`].
#[inline(always)]
pub unsafe fn __readl(addr: *const u32) -> u32 {
    unsafe { core::ptr::read_volatile(addr) }
}

/// Linux `__readq`, also exported as `__raw_readq`.
///
/// # Safety
/// See [`readb`].
#[inline(always)]
pub unsafe fn __readq(addr: *const u64) -> u64 {
    unsafe { core::ptr::read_volatile(addr) }
}

/// Linux `__writeb`, also exported as `__raw_writeb`.
///
/// # Safety
/// See [`writeb`].
#[inline(always)]
pub unsafe fn __writeb(val: u8, addr: *mut u8) {
    unsafe { core::ptr::write_volatile(addr, val) }
}

/// Linux `__writew`, also exported as `__raw_writew`.
///
/// # Safety
/// See [`writeb`].
#[inline(always)]
pub unsafe fn __writew(val: u16, addr: *mut u16) {
    unsafe { core::ptr::write_volatile(addr, val) }
}

/// Linux `__writel`, also exported as `__raw_writel`.
///
/// # Safety
/// See [`writeb`].
#[inline(always)]
pub unsafe fn __writel(val: u32, addr: *mut u32) {
    unsafe { core::ptr::write_volatile(addr, val) }
}

/// Linux `__writeq`, also exported as `__raw_writeq`.
///
/// # Safety
/// See [`writeb`].
#[inline(always)]
pub unsafe fn __writeq(val: u64, addr: *mut u64) {
    unsafe { core::ptr::write_volatile(addr, val) }
}

/// Linux `readb_relaxed`.
///
/// # Safety
/// See [`readb`].
#[inline(always)]
pub unsafe fn readb_relaxed(addr: *const u8) -> u8 {
    unsafe { __readb(addr) }
}

/// Linux `readw_relaxed`.
///
/// # Safety
/// See [`readw`].
#[inline(always)]
pub unsafe fn readw_relaxed(addr: *const u16) -> u16 {
    unsafe { __readw(addr) }
}

/// Linux `readl_relaxed`.
///
/// # Safety
/// See [`readl`].
#[inline(always)]
pub unsafe fn readl_relaxed(addr: *const u32) -> u32 {
    unsafe { __readl(addr) }
}

/// Linux `readq_relaxed`.
///
/// # Safety
/// See [`readq`].
#[inline(always)]
pub unsafe fn readq_relaxed(addr: *const u64) -> u64 {
    unsafe { __readq(addr) }
}

/// Linux `writeb_relaxed`.
///
/// # Safety
/// See [`writeb`].
#[inline(always)]
pub unsafe fn writeb_relaxed(val: u8, addr: *mut u8) {
    unsafe { __writeb(val, addr) }
}

/// Linux `writew_relaxed`.
///
/// # Safety
/// See [`writew`].
#[inline(always)]
pub unsafe fn writew_relaxed(val: u16, addr: *mut u16) {
    unsafe { __writew(val, addr) }
}

/// Linux `writel_relaxed`.
///
/// # Safety
/// See [`writel`].
#[inline(always)]
pub unsafe fn writel_relaxed(val: u32, addr: *mut u32) {
    unsafe { __writel(val, addr) }
}

/// Linux `writeq_relaxed`.
///
/// # Safety
/// See [`writeq`].
#[inline(always)]
pub unsafe fn writeq_relaxed(val: u64, addr: *mut u64) {
    unsafe { __writeq(val, addr) }
}

/// Linux `__raw_readb`.
///
/// # Safety
/// See [`readb`].
#[inline(always)]
pub unsafe fn __raw_readb(addr: *const u8) -> u8 {
    unsafe { __readb(addr) }
}

/// Linux `__raw_readw`.
///
/// # Safety
/// See [`readw`].
#[inline(always)]
pub unsafe fn __raw_readw(addr: *const u16) -> u16 {
    unsafe { __readw(addr) }
}

/// Linux `__raw_readl`.
///
/// # Safety
/// See [`readl`].
#[inline(always)]
pub unsafe fn __raw_readl(addr: *const u32) -> u32 {
    unsafe { __readl(addr) }
}

/// Linux `__raw_readq`.
///
/// # Safety
/// See [`readq`].
#[inline(always)]
pub unsafe fn __raw_readq(addr: *const u64) -> u64 {
    unsafe { __readq(addr) }
}

/// Linux `__raw_writeb`.
///
/// # Safety
/// See [`writeb`].
#[inline(always)]
pub unsafe fn __raw_writeb(val: u8, addr: *mut u8) {
    unsafe { __writeb(val, addr) }
}

/// Linux `__raw_writew`.
///
/// # Safety
/// See [`writew`].
#[inline(always)]
pub unsafe fn __raw_writew(val: u16, addr: *mut u16) {
    unsafe { __writew(val, addr) }
}

/// Linux `__raw_writel`.
///
/// # Safety
/// See [`writel`].
#[inline(always)]
pub unsafe fn __raw_writel(val: u32, addr: *mut u32) {
    unsafe { __writel(val, addr) }
}

/// Linux `__raw_writeq`.
///
/// # Safety
/// See [`writeq`].
#[inline(always)]
pub unsafe fn __raw_writeq(val: u64, addr: *mut u64) {
    unsafe { __writeq(val, addr) }
}

/// Linux `memcpy_fromio`.
///
/// # Safety
/// `dst` must cover `count` writable RAM bytes; `src` must cover `count`
/// readable MMIO bytes.
#[inline]
pub unsafe fn memcpy_fromio(dst: *mut u8, src: *const u8, count: usize) {
    unsafe {
        crate::arch::x86::lib::iomem::memcpy_fromio(dst, src, count);
    }
}

/// Linux `memcpy_toio`.
///
/// # Safety
/// `dst` must cover `count` writable MMIO bytes; `src` must cover `count`
/// readable RAM bytes.
#[inline]
pub unsafe fn memcpy_toio(dst: *mut u8, src: *const u8, count: usize) {
    unsafe {
        crate::arch::x86::lib::iomem::memcpy_toio(dst, src, count);
    }
}

/// Linux `memset_io`.
///
/// # Safety
/// `addr` must cover `count` writable MMIO bytes.
#[inline]
pub unsafe fn memset_io(addr: *mut u8, val: i32, count: usize) {
    unsafe {
        crate::arch::x86::lib::iomem::memset_io(addr, val as u8, count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINUX_IO_H: &str = include_str!("../../../../../vendor/linux/arch/x86/include/asm/io.h");

    #[test]
    fn linux_source_contains_io_parity_anchors() {
        assert!(LINUX_IO_H.contains("build_mmio_read(readb"));
        assert!(LINUX_IO_H.contains("#define readb_relaxed(a) __readb(a)"));
        assert!(LINUX_IO_H.contains("#define __raw_readb __readb"));
        assert!(LINUX_IO_H.contains("void memcpy_fromio"));
        assert!(LINUX_IO_H.contains("void memcpy_toio"));
        assert!(LINUX_IO_H.contains("void memset_io"));
        assert!(LINUX_IO_H.contains("static inline void outs##bwl"));
        assert!(LINUX_IO_H.contains("static inline void out##bwl##_p"));
        assert!(LINUX_IO_H.contains("BUILDIO(b, u8)"));
    }

    #[test]
    fn mmio_read_write_raw_and_relaxed_round_trip() {
        let mut byte = 0u8;
        unsafe {
            writeb(0x5a, &mut byte);
            assert_eq!(readb(&byte), 0x5a);
            writeb_relaxed(0xa5, &mut byte);
            assert_eq!(readb_relaxed(&byte), 0xa5);
            __raw_writeb(0x11, &mut byte);
            assert_eq!(__raw_readb(&byte), 0x11);
        }

        let mut word = 0u16;
        let mut dword = 0u32;
        let mut qword = 0u64;
        unsafe {
            writew_relaxed(0x1234, &mut word);
            writel_relaxed(0x89ab_cdef, &mut dword);
            writeq_relaxed(0x0123_4567_89ab_cdef, &mut qword);
            assert_eq!(readw_relaxed(&word), 0x1234);
            assert_eq!(readl_relaxed(&dword), 0x89ab_cdef);
            assert_eq!(readq_relaxed(&qword), 0x0123_4567_89ab_cdef);
        }
    }

    #[test]
    fn io_copy_helpers_delegate_and_preserve_bytes() {
        let src = [1u8, 2, 3, 4, 5, 6, 7];
        let mut mmio = [0u8; 7];
        let mut dst = [0u8; 7];
        unsafe {
            memcpy_toio(mmio.as_mut_ptr(), src.as_ptr(), src.len());
            memcpy_fromio(dst.as_mut_ptr(), mmio.as_ptr(), dst.len());
        }
        assert_eq!(mmio, src);
        assert_eq!(dst, src);
    }

    #[test]
    fn memset_io_uses_low_byte_like_linux_int_value() {
        let mut mmio = [0u8; 9];
        unsafe {
            memset_io(mmio.as_mut_ptr(), 0x1234_56a5, mmio.len());
        }
        assert_eq!(mmio, [0xa5; 9]);
    }
}
