//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/kernel
//! linux-source: vendor/linux/arch/x86/entry/vdso/vma.c
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! vDSO — virtual dynamic shared object mapped into every user process.
//!
//! Reserves the Linux x86 vDSO layout, maps vvar/vclock pages, and publishes
//! the base via `AT_SYSINFO_EHDR`. Remaining work vs Linux for `complete`: the
//! vDSO image is a structural placeholder with no `PT_DYNAMIC`, symbol table,
//! or real `__vdso_*` text (gettimeofday/clock_gettime/getcpu/time fast paths).
//!
//! ABI parity with vendor/linux/arch/x86/entry/vdso/. The kernel-side job is
//! to reserve the Linux x86 layout, map vvar/vclock pages before the text
//! image, and publish the vDSO ELF base through `AT_SYSINFO_EHDR`.
//!
//! References:
//! - vendor/linux/arch/x86/entry/vdso/vma.c
//!
use core::sync::atomic::{AtomicU32, AtomicU64};

use crate::mm::buddy::is_buddy_ready;
use crate::mm::frame::PAGE_SIZE;
use crate::mm::mm_types::{MmStruct, VmAreaStruct};
use crate::mm::mmap::{
    MAP_ANONYMOUS, MAP_FIXED_NOREPLACE, MAP_PRIVATE, PROT_EXEC, PROT_READ, PROT_WRITE, do_mmap,
    do_munmap, get_unmapped_area,
};
use crate::mm::mprotect::do_mprotect;
use crate::mm::vm_flags::{VM_DONTCOPY, VM_DONTDUMP, VM_IO, VM_PFNMAP, VmFlags};
use crate::mm::vma::find_vma;

/// `struct vsyscall_gtod_data` — kernel-shared time data the vDSO reads.
/// Byte-identical to vendor/linux/arch/x86/include/asm/vgtod.h.
#[repr(C)]
pub struct VsyscallGtodData {
    pub seq: AtomicU32,
    pub clock_mode: i32, // VDSO_CLOCKMODE_*
    pub cycle_last: AtomicU64,
    pub mask: u64,
    pub mult: u32,
    pub shift: u32,
    pub wall_time_sec: AtomicU64,
    pub wall_time_nsec: AtomicU64,
    pub monotonic_time_sec: AtomicU64,
    pub monotonic_time_nsec: AtomicU64,
    pub wall_to_monotonic_sec: i64,
    pub wall_to_monotonic_nsec: i64,
    pub tai_offset: i32,
    pub _pad: [u8; 4],
}

/// VDSO_CLOCKMODE_* — selects fastpath vs syscall fallback.
pub const VDSO_CLOCKMODE_NONE: i32 = 0;
pub const VDSO_CLOCKMODE_TSC: i32 = 1;
pub const VDSO_CLOCKMODE_PVCLOCK: i32 = 2;

/// Linux x86 reserves six pages before the vDSO text for vvar data.
///
/// Ref: vendor/linux/arch/x86/include/asm/vdso/vsyscall.h::__VDSO_PAGES
pub const VDSO_VVAR_PAGES: u64 = 6;

/// Linux x86 reserves two of the vvar pages for pvclock/hvclock mappings.
///
/// Ref: vendor/linux/arch/x86/include/asm/vdso/vsyscall.h::VDSO_NR_VCLOCK_PAGES
pub const VDSO_VCLOCK_PAGES: u64 = 2;

/// Lupos currently ships one page of vDSO ELF text/metadata.
pub const VDSO_TEXT_PAGES: u64 = 1;

pub const VDSO_IMAGE_SIZE: u64 = VDSO_TEXT_PAGES * PAGE_SIZE as u64;
pub const VDSO_MAPPING_SIZE: u64 = (VDSO_VVAR_PAGES + VDSO_TEXT_PAGES) * PAGE_SIZE as u64;

/// vDSO64 symbol availability in the current Lupos image.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VdsoSymbol {
    ClockGettime,
    Getcpu,
    Getrandom,
}

pub const fn vdso64_symbol_enabled(symbol: VdsoSymbol) -> bool {
    match symbol {
        VdsoSymbol::ClockGettime | VdsoSymbol::Getcpu | VdsoSymbol::Getrandom => false,
    }
}

/// Whether the current minimal vDSO is safe to advertise through auxv.
///
/// The login userland uses musl's dynamic loader, which treats
/// `AT_SYSINFO_EHDR` as a normal dynamic object. The current one-page image is
/// only a structural placeholder and has no `PT_DYNAMIC`, symbol table, or
/// version metadata, so exec must omit it until a complete vDSO is available.
pub const fn publish_vdso_to_userland() -> bool {
    false
}

/// Return the vDSO ELF header address to publish in `AT_SYSINFO_EHDR`.
///
/// The mapping helper remains available for vDSO-specific kernel tests, but
/// normal exec currently returns 0 so libc falls back to real syscalls.
pub unsafe fn exec_vdso_ehdr(mm: *mut MmStruct) -> Result<u64, i32> {
    if publish_vdso_to_userland() {
        Ok(unsafe { arch_setup_additional_pages(mm)? }.vdso_start)
    } else {
        let _ = mm;
        Ok(0)
    }
}

/// Global shared gtod data — written by the kernel timer tick under a
/// seqcount, read locklessly by the vDSO.  In M60 the seq stays at 0.
pub static VSYSCALL_GTOD_DATA: VsyscallGtodData = VsyscallGtodData {
    seq: AtomicU32::new(0),
    clock_mode: VDSO_CLOCKMODE_NONE,
    cycle_last: AtomicU64::new(0),
    mask: 0,
    mult: 0,
    shift: 0,
    wall_time_sec: AtomicU64::new(0),
    wall_time_nsec: AtomicU64::new(0),
    monotonic_time_sec: AtomicU64::new(0),
    monotonic_time_nsec: AtomicU64::new(0),
    wall_to_monotonic_sec: 0,
    wall_to_monotonic_nsec: 0,
    tai_offset: 0,
    _pad: [0; 4],
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VdsoMapping {
    pub vvar_start: u64,
    pub vclock_start: u64,
    pub vdso_start: u64,
    pub vdso_len: u64,
}

/// Map Linux-shaped vvar/vclock/vDSO VMAs into a process mm.
///
/// The layout follows `map_vdso()` in Linux:
/// `[vvar pages][vclock pages][vdso ELF image]`, with `AT_SYSINFO_EHDR`
/// pointing at `vdso_start`.
///
/// Ref: vendor/linux/arch/x86/entry/vdso/vma.c::arch_setup_additional_pages
/// Ref: vendor/linux/arch/x86/include/asm/elf.h::ARCH_DLINFO
///
/// # Safety
/// `mm` must be exclusively accessible, equivalent to Linux's mmap write lock.
pub unsafe fn arch_setup_additional_pages(mm: *mut MmStruct) -> Result<VdsoMapping, i32> {
    const EINVAL: i32 = -22;
    if mm.is_null() {
        return Err(EINVAL);
    }

    let mm_ref = unsafe { &mut *mm };
    let base = unsafe { get_unmapped_area(mm_ref, 0, VDSO_MAPPING_SIZE, 0)? };
    let vvar_start = base;
    let vclock_start = base + (VDSO_VVAR_PAGES - VDSO_VCLOCK_PAGES) * PAGE_SIZE as u64;
    let vdso_start = base + VDSO_VVAR_PAGES * PAGE_SIZE as u64;

    unsafe {
        do_mmap(
            mm_ref,
            vvar_start,
            VDSO_VVAR_PAGES * PAGE_SIZE as u64,
            PROT_READ,
            MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE,
            0,
            0,
        )?;
        mark_vma_special(
            mm_ref,
            vvar_start,
            VM_IO | VM_PFNMAP | VM_DONTCOPY | VM_DONTDUMP,
        )?;

        if let Err(err) = do_mmap(
            mm_ref,
            vdso_start,
            VDSO_IMAGE_SIZE,
            PROT_READ | PROT_WRITE | PROT_EXEC,
            MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE,
            0,
            0,
        ) {
            let _ = do_munmap(mm_ref, vvar_start, VDSO_VVAR_PAGES * PAGE_SIZE as u64);
            return Err(err);
        }
    }

    let image = build_minimal_vdso_elf();
    let write_result = if unsafe { (*mm).pgd } != 0 && is_buddy_ready() {
        unsafe { crate::kernel::exec::user_write(mm, vdso_start, &image) }
    } else {
        Ok(())
    };
    if let Err(err) = write_result {
        unsafe {
            let _ = do_munmap(mm_ref, vvar_start, VDSO_MAPPING_SIZE);
        }
        return Err(err);
    }

    unsafe {
        if let Err(err) = do_mprotect(mm_ref, vdso_start, VDSO_IMAGE_SIZE, PROT_READ | PROT_EXEC) {
            let _ = do_munmap(mm_ref, vvar_start, VDSO_MAPPING_SIZE);
            return Err(err);
        }
    }

    Ok(VdsoMapping {
        vvar_start,
        vclock_start,
        vdso_start,
        vdso_len: VDSO_IMAGE_SIZE,
    })
}

fn mark_vma_special(mm: &MmStruct, addr: u64, extra: VmFlags) -> Result<(), i32> {
    let Some(vma) = find_vma(mm, addr) else {
        return Err(-12);
    };
    let vma = unsafe { &mut *(vma as *mut VmAreaStruct) };
    if !vma.contains(addr) {
        return Err(-12);
    }
    vma.vm_flags |= extra;
    Ok(())
}

pub fn build_minimal_vdso_elf() -> [u8; PAGE_SIZE] {
    const ET_DYN: u16 = 3;
    const EM_X86_64: u16 = 62;
    const EV_CURRENT: u32 = 1;
    const PT_LOAD: u32 = 1;
    const PF_X: u32 = 1;
    const PF_R: u32 = 4;
    const ELF_HEADER_SIZE: u16 = 64;
    const PHDR_SIZE: u16 = 56;

    let mut image = [0u8; PAGE_SIZE];
    image[0..4].copy_from_slice(b"\x7fELF");
    image[4] = 2; // ELFCLASS64
    image[5] = 1; // ELFDATA2LSB
    image[6] = 1; // EV_CURRENT

    put_u16(&mut image, 16, ET_DYN);
    put_u16(&mut image, 18, EM_X86_64);
    put_u32(&mut image, 20, EV_CURRENT);
    put_u64(&mut image, 32, ELF_HEADER_SIZE as u64);
    put_u16(&mut image, 52, ELF_HEADER_SIZE);
    put_u16(&mut image, 54, PHDR_SIZE);
    put_u16(&mut image, 56, 1);

    let ph = ELF_HEADER_SIZE as usize;
    put_u32(&mut image, ph, PT_LOAD);
    put_u32(&mut image, ph + 4, PF_R | PF_X);
    put_u64(&mut image, ph + 32, PAGE_SIZE as u64);
    put_u64(&mut image, ph + 40, PAGE_SIZE as u64);
    put_u64(&mut image, ph + 48, PAGE_SIZE as u64);
    image
}

fn put_u16(buf: &mut [u8], off: usize, val: u16) {
    buf[off..off + 2].copy_from_slice(&val.to_le_bytes());
}

fn put_u32(buf: &mut [u8], off: usize, val: u32) {
    buf[off..off + 4].copy_from_slice(&val.to_le_bytes());
}

fn put_u64(buf: &mut [u8], off: usize, val: u64) {
    buf[off..off + 8].copy_from_slice(&val.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::Ordering;

    #[test]
    fn vsyscall_gtod_data_first_field_is_seq() {
        assert_eq!(core::mem::offset_of!(VsyscallGtodData, seq), 0);
    }

    #[test]
    fn vsyscall_gtod_data_seq_starts_zero() {
        assert_eq!(VSYSCALL_GTOD_DATA.seq.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn arch_setup_additional_pages_installs_linux_shaped_layout() {
        let mut mm = MmStruct::new(0);
        let mapping = unsafe { arch_setup_additional_pages(&mut mm) }.unwrap();
        assert_eq!(
            mapping.vclock_start,
            mapping.vvar_start + 4 * PAGE_SIZE as u64
        );
        assert_eq!(
            mapping.vdso_start,
            mapping.vvar_start + 6 * PAGE_SIZE as u64
        );
        assert_eq!(mapping.vdso_len, PAGE_SIZE as u64);
        assert_eq!(mm.map_count, 2);

        let vvar = find_vma(&mm, mapping.vvar_start).unwrap();
        assert_ne!(unsafe { (*vvar).vm_flags } & VM_IO, 0);
        assert_ne!(unsafe { (*vvar).vm_flags } & VM_DONTDUMP, 0);
    }

    #[test]
    fn minimal_vdso_image_is_elf64_dyn() {
        let image = build_minimal_vdso_elf();
        assert_eq!(&image[0..4], b"\x7fELF");
        assert_eq!(image[4], 2);
        assert_eq!(u16::from_le_bytes([image[16], image[17]]), 3);
        assert_eq!(u16::from_le_bytes([image[18], image[19]]), 62);
    }

    #[test]
    fn vdso64_symbols_reflect_current_minimal_image() {
        assert!(!vdso64_symbol_enabled(VdsoSymbol::ClockGettime));
        assert!(!vdso64_symbol_enabled(VdsoSymbol::Getcpu));
        assert!(!vdso64_symbol_enabled(VdsoSymbol::Getrandom));
    }

    #[test]
    fn minimal_vdso_is_not_published_to_exec_auxv() {
        let mut mm = MmStruct::new(0);
        let ehdr = unsafe { exec_vdso_ehdr(&mut mm) }.expect("vdso auxv decision");
        assert_eq!(ehdr, 0);
        assert!(!publish_vdso_to_userland());
    }
}
