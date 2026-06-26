//! linux-parity: complete
//! linux-source: vendor/linux/lib/iomem_copy.c
//! test-origin: linux:vendor/linux/lib/iomem_copy.c
//! Generic raw MMIO byte and word copy helpers.

use core::ffi::c_void;
use core::mem::size_of;
use core::ptr::{read_unaligned, read_volatile, write_unaligned, write_volatile};

use crate::kernel::module::{export_symbol, find_symbol};

const WORD_SIZE: usize = size_of::<usize>();

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("memset_io", memset_io as usize, false);
    export_symbol_once("memcpy_fromio", memcpy_fromio as usize, false);
    export_symbol_once("memcpy_toio", memcpy_toio as usize, false);
}

#[inline]
const fn is_word_aligned(addr: usize) -> bool {
    addr & (WORD_SIZE - 1) == 0
}

#[inline]
const fn repeated_byte_word(val: i32) -> usize {
    (val as u8 as usize) * (usize::MAX / 0xff)
}

/// `memset_io()` - set a range of I/O memory to a constant byte value.
///
/// # Safety
/// `addr` must cover `count` writable MMIO bytes. Word writes require the
/// aligned portion of the MMIO range to accept native-width accesses.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memset_io(mut addr: *mut c_void, val: i32, mut count: usize) {
    let word = repeated_byte_word(val);

    while count != 0 && !is_word_aligned(addr as usize) {
        unsafe { write_volatile(addr.cast::<u8>(), val as u8) };
        addr = unsafe { addr.cast::<u8>().add(1).cast() };
        count -= 1;
    }

    while count >= WORD_SIZE {
        unsafe { write_volatile(addr.cast::<usize>(), word) };
        addr = unsafe { addr.cast::<u8>().add(WORD_SIZE).cast() };
        count -= WORD_SIZE;
    }

    while count != 0 {
        unsafe { write_volatile(addr.cast::<u8>(), val as u8) };
        addr = unsafe { addr.cast::<u8>().add(1).cast() };
        count -= 1;
    }
}

/// `memcpy_fromio()` - copy bytes from MMIO into RAM.
///
/// # Safety
/// `src` must cover `count` readable MMIO bytes and `dst` must cover `count`
/// writable RAM bytes. The regions must not overlap.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcpy_fromio(
    mut dst: *mut c_void,
    mut src: *const c_void,
    mut count: usize,
) {
    while count != 0 && !is_word_aligned(src as usize) {
        unsafe { *dst.cast::<u8>() = read_volatile(src.cast::<u8>()) };
        src = unsafe { src.cast::<u8>().add(1).cast() };
        dst = unsafe { dst.cast::<u8>().add(1).cast() };
        count -= 1;
    }

    while count >= WORD_SIZE {
        let val = unsafe { read_volatile(src.cast::<usize>()) };
        unsafe { write_unaligned(dst.cast::<usize>(), val) };
        src = unsafe { src.cast::<u8>().add(WORD_SIZE).cast() };
        dst = unsafe { dst.cast::<u8>().add(WORD_SIZE).cast() };
        count -= WORD_SIZE;
    }

    while count != 0 {
        unsafe { *dst.cast::<u8>() = read_volatile(src.cast::<u8>()) };
        src = unsafe { src.cast::<u8>().add(1).cast() };
        dst = unsafe { dst.cast::<u8>().add(1).cast() };
        count -= 1;
    }
}

/// `memcpy_toio()` - copy bytes from RAM into MMIO.
///
/// # Safety
/// `dst` must cover `count` writable MMIO bytes and `src` must cover `count`
/// readable RAM bytes. The regions must not overlap.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcpy_toio(
    mut dst: *mut c_void,
    mut src: *const c_void,
    mut count: usize,
) {
    while count != 0 && !is_word_aligned(dst as usize) {
        unsafe { write_volatile(dst.cast::<u8>(), *src.cast::<u8>()) };
        src = unsafe { src.cast::<u8>().add(1).cast() };
        dst = unsafe { dst.cast::<u8>().add(1).cast() };
        count -= 1;
    }

    while count >= WORD_SIZE {
        let val = unsafe { read_unaligned(src.cast::<usize>()) };
        unsafe { write_volatile(dst.cast::<usize>(), val) };
        src = unsafe { src.cast::<u8>().add(WORD_SIZE).cast() };
        dst = unsafe { dst.cast::<u8>().add(WORD_SIZE).cast() };
        count -= WORD_SIZE;
    }

    while count != 0 {
        unsafe { write_volatile(dst.cast::<u8>(), *src.cast::<u8>()) };
        src = unsafe { src.cast::<u8>().add(1).cast() };
        dst = unsafe { dst.cast::<u8>().add(1).cast() };
        count -= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aligned_offset(ptr: *const u8) -> usize {
        let rem = ptr as usize & (WORD_SIZE - 1);
        if rem == 0 { 0 } else { WORD_SIZE - rem }
    }

    #[test]
    fn iomem_copy_source_matches_linux_alignment_and_exports() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/iomem_copy.c"
        ));
        assert!(source.contains("void memset_io(volatile void __iomem *addr"));
        assert!(source.contains("qc *= ~0UL / 0xff;"));
        assert!(source.contains("while (count && !IS_ALIGNED((long)addr, sizeof(long)))"));
        assert!(source.contains("__raw_writeb(val, addr);"));
        assert!(
            source.contains("__raw_writeq(qc, addr);")
                || source.contains("__raw_writel(qc, addr);")
        );
        assert!(source.contains("void memcpy_fromio(void *dst"));
        assert!(
            source.contains("long val = __raw_readq(src);")
                || source.contains("long val = __raw_readl(src);")
        );
        assert!(source.contains("put_unaligned(val, (long *)dst);"));
        assert!(source.contains("void memcpy_toio(volatile void __iomem *dst"));
        assert!(source.contains("long val = get_unaligned((long *)src);"));
        assert!(source.contains("EXPORT_SYMBOL(memset_io);"));
        assert!(source.contains("EXPORT_SYMBOL(memcpy_fromio);"));
        assert!(source.contains("EXPORT_SYMBOL(memcpy_toio);"));
    }

    #[test]
    fn memset_io_handles_unaligned_prefix_word_loop_and_tail() {
        let mut mmio = [0u8; 64];
        let start = aligned_offset(mmio.as_ptr()) + 1;
        let count = WORD_SIZE * 2 + 3;

        unsafe { memset_io(mmio.as_mut_ptr().add(start).cast(), 0x5a, count) };

        assert_eq!(&mmio[..start], &[0u8; 64][..start]);
        assert!(mmio[start..start + count].iter().all(|&byte| byte == 0x5a));
        assert!(mmio[start + count..].iter().all(|&byte| byte == 0));
    }

    #[test]
    fn memcpy_fromio_preserves_bytes_with_unaligned_src_and_dst() {
        let mut src = [0u8; 80];
        for (i, byte) in src.iter_mut().enumerate() {
            *byte = (i as u8).wrapping_mul(17).wrapping_add(3);
        }
        let mut dst = [0u8; 80];
        let src_start = aligned_offset(src.as_ptr()) + 1;
        let dst_start = 3;
        let count = WORD_SIZE * 3 + 5;

        unsafe {
            memcpy_fromio(
                dst.as_mut_ptr().add(dst_start).cast(),
                src.as_ptr().add(src_start).cast(),
                count,
            )
        };

        assert_eq!(
            &dst[dst_start..dst_start + count],
            &src[src_start..src_start + count]
        );
        assert!(dst[..dst_start].iter().all(|&byte| byte == 0));
        assert!(dst[dst_start + count..].iter().all(|&byte| byte == 0));
    }

    #[test]
    fn memcpy_toio_preserves_bytes_with_unaligned_src_and_dst() {
        let mut src = [0u8; 80];
        for (i, byte) in src.iter_mut().enumerate() {
            *byte = (0xe0u8).wrapping_sub(i as u8);
        }
        let mut dst = [0u8; 80];
        let src_start = 5;
        let dst_start = aligned_offset(dst.as_ptr()) + 1;
        let count = WORD_SIZE * 3 + 5;

        unsafe {
            memcpy_toio(
                dst.as_mut_ptr().add(dst_start).cast(),
                src.as_ptr().add(src_start).cast(),
                count,
            )
        };

        assert_eq!(
            &dst[dst_start..dst_start + count],
            &src[src_start..src_start + count]
        );
        assert!(dst[..dst_start].iter().all(|&byte| byte == 0));
        assert!(dst[dst_start + count..].iter().all(|&byte| byte == 0));
    }

    #[test]
    fn iomem_copy_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("memset_io"),
            Some(memset_io as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("memcpy_fromio"),
            Some(memcpy_fromio as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("memcpy_toio"),
            Some(memcpy_toio as usize)
        );
    }
}
