//! linux-parity: complete
//! linux-source: vendor/linux/lib/stmp_device.c
//! test-origin: linux:vendor/linux/lib/stmp_device.c
//! STMP-style reset register helper.

use core::ffi::c_void;
use core::ptr::{read_volatile, write_volatile};

use crate::include::uapi::errno::ETIMEDOUT;
use crate::kernel::module::{export_symbol, find_symbol};

pub const STMP_OFFSET_REG_SET: usize = 0x4;
pub const STMP_OFFSET_REG_CLR: usize = 0x8;
pub const STMP_OFFSET_REG_TOG: usize = 0xc;
pub const STMP_MODULE_CLKGATE: u32 = 1 << 30;
pub const STMP_MODULE_SFTRST: u32 = 1 << 31;
const STMP_POLL_TIMEOUT: usize = 0x400;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("stmp_reset_block", stmp_reset_block as usize, false);
}

unsafe fn reg(base: *mut c_void, offset: usize) -> *mut u32 {
    unsafe { base.cast::<u8>().add(offset).cast::<u32>() }
}

unsafe fn readl(base: *mut c_void) -> u32 {
    unsafe { read_volatile(reg(base, 0)) }
}

unsafe fn writel(value: u32, addr: *mut u32) {
    unsafe { write_volatile(addr, value) };
}

unsafe fn write_set(base: *mut c_void, mask: u32) {
    unsafe {
        writel(mask, reg(base, STMP_OFFSET_REG_SET));
        let current = readl(base);
        writel(current | mask, reg(base, 0));
    }
}

unsafe fn write_clr(base: *mut c_void, mask: u32) {
    unsafe {
        writel(mask, reg(base, STMP_OFFSET_REG_CLR));
        let current = readl(base);
        writel(current & !mask, reg(base, 0));
    }
}

unsafe fn stmp_clear_poll_bit(addr: *mut c_void, mask: u32) -> i32 {
    let mut timeout = STMP_POLL_TIMEOUT;
    unsafe { write_clr(addr, mask) };
    while unsafe { readl(addr) } & mask != 0 && timeout != 0 {
        timeout -= 1;
    }
    if timeout == 0 { 1 } else { 0 }
}

pub unsafe extern "C" fn stmp_reset_block(reset_addr: *mut c_void) -> i32 {
    if reset_addr.is_null() {
        return -ETIMEDOUT;
    }

    let mut timeout = STMP_POLL_TIMEOUT;

    if unsafe { stmp_clear_poll_bit(reset_addr, STMP_MODULE_SFTRST) } != 0 {
        return -ETIMEDOUT;
    }

    unsafe {
        write_clr(reset_addr, STMP_MODULE_CLKGATE);
        write_set(reset_addr, STMP_MODULE_SFTRST);
        write_set(reset_addr, STMP_MODULE_CLKGATE);
    }

    while unsafe { readl(reset_addr) } & STMP_MODULE_CLKGATE == 0 && timeout != 0 {
        timeout -= 1;
    }
    if timeout == 0 {
        return -ETIMEDOUT;
    }

    if unsafe { stmp_clear_poll_bit(reset_addr, STMP_MODULE_SFTRST) } != 0 {
        return -ETIMEDOUT;
    }
    if unsafe { stmp_clear_poll_bit(reset_addr, STMP_MODULE_CLKGATE) } != 0 {
        return -ETIMEDOUT;
    }

    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stmp_reset_block_follows_linux_set_clear_sequence() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/stmp_device.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/stmp_device.h"
        ));
        assert!(source.contains("STMP_MODULE_CLKGATE\t(1 << 30)"));
        assert!(source.contains("STMP_MODULE_SFTRST\t(1 << 31)"));
        assert!(source.contains("writel(mask, addr + STMP_OFFSET_REG_CLR);"));
        assert!(source.contains("writel(STMP_MODULE_SFTRST, reset_addr + STMP_OFFSET_REG_SET);"));
        assert!(source.contains("return -ETIMEDOUT;"));
        assert!(header.contains("#define STMP_OFFSET_REG_SET\t0x4"));

        let mut regs = [0u32; 4];
        let ret = unsafe { stmp_reset_block(regs.as_mut_ptr().cast()) };
        assert_eq!(ret, 0);
        assert_eq!(regs[0] & (STMP_MODULE_CLKGATE | STMP_MODULE_SFTRST), 0);
        assert_eq!(regs[STMP_OFFSET_REG_SET / 4], STMP_MODULE_CLKGATE);
        assert_eq!(regs[STMP_OFFSET_REG_CLR / 4], STMP_MODULE_CLKGATE);
    }
}
