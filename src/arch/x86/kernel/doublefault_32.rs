//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/doublefault_32.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/doublefault_32.c
//! x86-32 double-fault TSS model.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/doublefault_32.c

use crate::arch::x86::mm::paging::PAGE_SIZE;
use crate::include::uapi::errno::EINVAL;

pub const IA32_PAGE_OFFSET: u32 = 0xc000_0000;
pub const IA32_MAXMEM: u32 = 0x3800_0000;

pub const X86_EFLAGS_FIXED: u32 = 1 << 1;
pub const GDT_ENTRY_KERNEL_CS_32: u16 = 12;
pub const GDT_ENTRY_KERNEL_DS_32: u16 = 13;
pub const GDT_ENTRY_DEFAULT_USER_DS_32: u16 = 15;
pub const GDT_ENTRY_DOUBLEFAULT_TSS: u16 = 31;

pub const __KERNEL_CS: u16 = GDT_ENTRY_KERNEL_CS_32 * 8;
pub const __KERNEL_DS: u16 = GDT_ENTRY_KERNEL_DS_32 * 8;
pub const __USER_DS: u16 = GDT_ENTRY_DEFAULT_USER_DS_32 * 8 + 3;
pub const __KERNEL_PERCPU: u16 = 0;
pub const IO_BITMAP_OFFSET_INVALID: u16 = 0xffff;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DoubleFaultTss {
    pub ldt: u16,
    pub io_bitmap_base: u16,
    pub ip: u32,
    pub sp: u32,
    pub flags: u32,
    pub es: u16,
    pub cs: u16,
    pub ss: u16,
    pub ds: u16,
    pub fs: u16,
    pub gs: u16,
    pub cr3: u32,
    pub ax: u32,
    pub bp: u32,
    pub di: u32,
    pub si: u32,
    pub dx: u32,
    pub cx: u32,
    pub bx: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DoubleFaultRegs {
    pub ss: u32,
    pub sp: u32,
    pub flags: u32,
    pub cs: u32,
    pub ip: u32,
    pub gs: u32,
    pub fs: u32,
    pub es: u32,
    pub ds: u32,
    pub ax: u32,
    pub bp: u32,
    pub di: u32,
    pub si: u32,
    pub dx: u32,
    pub cx: u32,
    pub bx: u32,
}

pub const fn ptr_ok(value: u32, page_offset: u32, maxmem: u32) -> bool {
    value > page_offset && value < page_offset + maxmem
}

pub const fn doublefault_stack_words(page_size: usize, tss_size: usize) -> Result<usize, i32> {
    if tss_size >= page_size {
        Err(EINVAL)
    } else {
        Ok((page_size - tss_size) / core::mem::size_of::<u32>())
    }
}

pub const fn doublefault_stack_size() -> u64 {
    PAGE_SIZE
}

pub const fn doublefault_init_cpu_tss(
    stack_top: u32,
    handler_ip: u32,
    swapper_pg_dir_phys: u32,
) -> DoubleFaultTss {
    DoubleFaultTss {
        ldt: 0,
        io_bitmap_base: IO_BITMAP_OFFSET_INVALID,
        ip: handler_ip,
        sp: stack_top,
        flags: X86_EFLAGS_FIXED,
        es: __USER_DS,
        cs: __KERNEL_CS,
        ss: __KERNEL_DS,
        ds: __USER_DS,
        fs: __KERNEL_PERCPU,
        gs: 0,
        cr3: swapper_pg_dir_phys,
        ax: 0,
        bp: 0,
        di: 0,
        si: 0,
        dx: 0,
        cx: 0,
        bx: 0,
    }
}

pub const fn doublefault_shim_regs_from_tss(tss: DoubleFaultTss) -> DoubleFaultRegs {
    DoubleFaultRegs {
        ss: tss.ss as u32,
        sp: tss.sp,
        flags: tss.flags,
        cs: tss.cs as u32,
        ip: tss.ip,
        gs: tss.gs as u32,
        fs: tss.fs as u32,
        es: tss.es as u32,
        ds: tss.ds as u32,
        ax: tss.ax,
        bp: tss.bp,
        di: tss.di,
        si: tss.si,
        dx: tss.dx,
        cx: tss.cx,
        bx: tss.bx,
    }
}

pub const fn doublefault_gdt_selector() -> u16 {
    GDT_ENTRY_DOUBLEFAULT_TSS * 8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ptr_ok_matches_linux_exclusive_bounds() {
        assert!(!ptr_ok(IA32_PAGE_OFFSET, IA32_PAGE_OFFSET, IA32_MAXMEM));
        assert!(ptr_ok(IA32_PAGE_OFFSET + 1, IA32_PAGE_OFFSET, IA32_MAXMEM));
        assert!(!ptr_ok(
            IA32_PAGE_OFFSET + IA32_MAXMEM,
            IA32_PAGE_OFFSET,
            IA32_MAXMEM
        ));
    }

    #[test]
    fn doublefault_stack_is_one_page_with_tss_at_end() {
        assert_eq!(doublefault_stack_size(), PAGE_SIZE);
        assert_eq!(doublefault_stack_words(4096, 104), Ok(998));
        assert_eq!(doublefault_stack_words(4096, 4096), Err(EINVAL));
    }

    #[test]
    fn initial_tss_uses_linux_segment_constants() {
        let tss = doublefault_init_cpu_tss(0x8000, 0x1234, 0x9000);
        assert_eq!(tss.flags, X86_EFLAGS_FIXED);
        assert_eq!(tss.cs, 0x60);
        assert_eq!(tss.ss, 0x68);
        assert_eq!(tss.ds, 0x7b);
        assert_eq!(tss.ip, 0x1234);
    }

    #[test]
    fn shim_regs_copy_hardware_tss_fields() {
        let mut tss = doublefault_init_cpu_tss(0x8000, 0x1234, 0x9000);
        tss.ax = 1;
        tss.bx = 2;
        tss.cx = 3;
        let regs = doublefault_shim_regs_from_tss(tss);
        assert_eq!(regs.sp, 0x8000);
        assert_eq!(regs.ip, 0x1234);
        assert_eq!(regs.ax, 1);
        assert_eq!(regs.bx, 2);
        assert_eq!(regs.cx, 3);
    }
}
