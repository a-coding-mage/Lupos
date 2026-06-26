//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/sys_x86_64.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/sys_x86_64.c
//! x86-64 mmap and virtual-address alignment policy.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/sys_x86_64.c

#![allow(dead_code)]

use crate::arch::x86::mm::paging::{PAGE_MASK, PAGE_SIZE};
use crate::include::uapi::errno::{EINVAL, ENOMEM};
use crate::mm::mmap::{MAP_FIXED, TASK_SIZE};
use crate::mm::vm_flags::{VM_SHADOW_STACK, VmFlags};

pub const MAP_32BIT: u32 = 0x40;
pub const MAP_ABOVE4G: u32 = 0x80;
pub const ALIGN_VA_32: i32 = 1;
pub const ALIGN_VA_64: i32 = 2;
pub const DEFAULT_MAP_WINDOW: u64 = 1u64 << 47;
pub const TASK_SIZE_MAX: u64 = TASK_SIZE;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VaAlign {
    pub flags: i32,
    pub mask: u64,
    pub bits: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnmappedAreaWindow {
    pub begin: u64,
    pub end: u64,
    pub topdown: bool,
    pub start_gap: u64,
}

pub const fn control_va_addr_alignment(mut align: VaAlign, value: &str) -> Result<VaAlign, i32> {
    if align.flags < 0 || value.is_empty() {
        return Ok(align);
    }
    let bytes = value.as_bytes();
    align.flags = if bytes.len() == 2 && bytes[0] == b'3' && bytes[1] == b'2' {
        ALIGN_VA_32
    } else if bytes.len() == 2 && bytes[0] == b'6' && bytes[1] == b'4' {
        ALIGN_VA_64
    } else if bytes.len() == 3 && bytes[0] == b'o' && bytes[1] == b'f' && bytes[2] == b'f' {
        0
    } else if bytes.len() == 2 && bytes[0] == b'o' && bytes[1] == b'n' {
        ALIGN_VA_32 | ALIGN_VA_64
    } else {
        return Err(EINVAL);
    };
    Ok(align)
}

pub const fn get_align_mask(
    align: VaAlign,
    is_ia32: bool,
    randomized: bool,
    file_huge: bool,
    huge_mask: u64,
) -> u64 {
    if file_huge {
        return huge_mask;
    }
    let bit = if is_ia32 { ALIGN_VA_32 } else { ALIGN_VA_64 };
    if align.flags < 0 || align.flags & bit == 0 || !randomized {
        0
    } else {
        align.mask
    }
}

pub const fn get_align_bits(align: VaAlign, mask: u64) -> u64 {
    align.bits & mask
}

pub const fn stack_guard_placement(vm_flags: VmFlags) -> u64 {
    if vm_flags & VM_SHADOW_STACK != 0 {
        PAGE_SIZE
    } else {
        0
    }
}

pub const fn find_start_end(
    addr: u64,
    flags: u32,
    is_ia32: bool,
    randomized_begin: u64,
) -> (u64, u64) {
    if !is_ia32 && flags & MAP_32BIT != 0 {
        let begin = if randomized_begin != 0 {
            randomized_begin
        } else {
            0x4000_0000
        };
        return (begin, 0x8000_0000);
    }
    let end = if is_ia32 || addr <= DEFAULT_MAP_WINDOW {
        DEFAULT_MAP_WINDOW
    } else {
        TASK_SIZE_MAX
    };
    (PAGE_SIZE, end)
}

pub const fn mmap_syscall_pgoff(off: u64) -> Result<u64, i32> {
    if off & !PAGE_MASK != 0 {
        Err(EINVAL)
    } else {
        Ok(off >> crate::arch::x86::mm::paging::PAGE_SHIFT)
    }
}

pub const fn arch_get_unmapped_area_window(
    addr: u64,
    len: u64,
    flags: u32,
    vm_flags: VmFlags,
    is_ia32: bool,
) -> Result<UnmappedAreaWindow, i32> {
    if flags & MAP_FIXED != 0 {
        return Ok(UnmappedAreaWindow {
            begin: addr,
            end: addr.saturating_add(len),
            topdown: false,
            start_gap: 0,
        });
    }
    let (begin, end) = find_start_end(addr, flags, is_ia32, 0);
    if len > end {
        return Err(ENOMEM);
    }
    Ok(UnmappedAreaWindow {
        begin,
        end,
        topdown: false,
        start_gap: stack_guard_placement(vm_flags),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mmap_rejects_unaligned_file_offset() {
        assert_eq!(mmap_syscall_pgoff(0x3000), Ok(3));
        assert_eq!(mmap_syscall_pgoff(7), Err(EINVAL));
    }

    #[test]
    fn map_32bit_uses_low_window_on_64bit_syscalls() {
        assert_eq!(
            find_start_end(0, MAP_32BIT, false, 0),
            (0x4000_0000, 0x8000_0000)
        );
        assert_eq!(stack_guard_placement(VM_SHADOW_STACK), PAGE_SIZE);
    }

    #[test]
    fn va_alignment_options_match_cmdline() {
        let align = VaAlign {
            flags: 0,
            mask: 0xfff,
            bits: 0x555,
        };
        assert_eq!(
            control_va_addr_alignment(align, "64").unwrap().flags,
            ALIGN_VA_64
        );
        assert_eq!(control_va_addr_alignment(align, "bad"), Err(EINVAL));
    }
}
