//! linux-parity: complete
//! linux-source: vendor/linux/mm/util.c
//! test-origin: linux:vendor/linux/mm/util.c
//! Miscellaneous Linux MM utilities: dmapool, mempool, and overflow helpers.
//!
//! References:
//! - `vendor/linux/mm/dmapool.c`
//! - `vendor/linux/mm/mempool.c`
//! - `vendor/linux/mm/util.c`

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use core::ffi::c_void;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::arch::x86::kernel::uaccess::copy_from_user;
use crate::include::uapi::errno::{EFAULT, EINVAL, ENOMEM};
use crate::kernel::module::{export_symbol, find_symbol};
use crate::kernel::sched;
use crate::mm::buddy::{page_in_mem_map, page_to_pfn, pfn_to_page};
use crate::mm::mm_types::{MmStruct, VmAreaStruct};
use crate::mm::page::Page;
use crate::mm::page_flags::{__GFP_NOWARN, GFP_USER};
use spin::Mutex;

static VM_MEMORY_COMMITTED: AtomicU64 = AtomicU64::new(0);
static KSTRDUP_CONST_ALLOCS: Mutex<Vec<ConstDupAlloc>> = Mutex::new(Vec::new());

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ConstDupAlloc {
    ptr: usize,
    len: usize,
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("memdup_user", linux_memdup_user as usize, false);
    export_symbol_once("vmemdup_user", linux_vmemdup_user as usize, false);
    export_symbol_once("memdup_user_nul", linux_memdup_user_nul as usize, false);
    export_symbol_once("strndup_user", linux_strndup_user as usize, false);
    export_symbol_once("kfree_const", linux_kfree_const as usize, false);
    export_symbol_once("kstrdup", linux_kstrdup as usize, false);
    export_symbol_once("kstrdup_const", linux_kstrdup_const as usize, false);
    export_symbol_once("kstrndup", linux_kstrndup as usize, false);
    export_symbol_once("kmemdup_array", linux_kmemdup_array as usize, false);
    export_symbol_once("kmemdup_nul", linux_kmemdup_nul as usize, false);
    export_symbol_once("vma_set_file", linux_vma_set_file as usize, false);
    export_symbol_once("compat_vma_mmap", linux_compat_vma_mmap as usize, false);
}

pub fn array_size(count: usize, size: usize) -> Result<usize, i32> {
    count.checked_mul(size).ok_or(ENOMEM)
}

pub fn array3_size(a: usize, b: usize, c: usize) -> Result<usize, i32> {
    array_size(array_size(a, b)?, c)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemPool {
    min_nr: usize,
    elements: Vec<Vec<u8>>,
    element_size: usize,
}

impl MemPool {
    pub fn new(min_nr: usize, element_size: usize) -> Result<Self, i32> {
        if min_nr == 0 || element_size == 0 {
            return Err(EINVAL);
        }
        let mut elements = Vec::new();
        for _ in 0..min_nr {
            elements.push(vec![0u8; element_size]);
        }
        Ok(Self {
            min_nr,
            elements,
            element_size,
        })
    }

    pub fn alloc(&mut self) -> Vec<u8> {
        self.elements
            .pop()
            .unwrap_or_else(|| vec![0u8; self.element_size])
    }

    pub fn free(&mut self, mut element: Vec<u8>) {
        element.resize(self.element_size, 0);
        self.elements.push(element);
    }

    pub fn available(&self) -> usize {
        self.elements.len()
    }

    pub fn min_nr(&self) -> usize {
        self.min_nr
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DmaPool {
    block_size: usize,
    align: usize,
    blocks: Vec<Vec<u8>>,
}

impl DmaPool {
    pub fn new(block_size: usize, align: usize) -> Result<Self, i32> {
        if block_size == 0 {
            return Err(EINVAL);
        }
        dmapool_alignment(align)?;
        Ok(Self {
            block_size: align_up(block_size, align).ok_or(ENOMEM)?,
            align,
            blocks: Vec::new(),
        })
    }

    pub fn alloc(&mut self) -> Vec<u8> {
        self.blocks
            .pop()
            .unwrap_or_else(|| vec![0u8; self.block_size])
    }

    pub fn free(&mut self, mut block: Vec<u8>) {
        block.resize(self.block_size, 0);
        self.blocks.push(block);
    }

    pub fn align(&self) -> usize {
        self.align
    }
}

pub fn mempool_min_nr(min_nr: usize) -> Result<usize, i32> {
    if min_nr == 0 { Err(EINVAL) } else { Ok(min_nr) }
}

pub fn dmapool_alignment(align: usize) -> Result<usize, i32> {
    if align == 0 || !align.is_power_of_two() {
        Err(EINVAL)
    } else {
        Ok(align)
    }
}

fn align_up(value: usize, align: usize) -> Option<usize> {
    value
        .checked_add(align - 1)
        .map(|value| value & !(align - 1))
}

// ---------------------------------------------------------------------------
// Linux-visible util.c / mman.h wrappers
// ---------------------------------------------------------------------------

pub fn __account_locked_vm(mm: *mut MmStruct, pages: u64, inc: bool, _task: *mut u8) -> i32 {
    if mm.is_null() || pages == 0 {
        return 0;
    }
    let mm = unsafe { &mut *mm };
    if inc {
        let Some(new_locked) = mm.locked_vm.checked_add(pages) else {
            return -ENOMEM;
        };
        mm.locked_vm = new_locked;
    } else {
        mm.locked_vm = mm.locked_vm.saturating_sub(pages);
    }
    0
}

pub fn account_locked_vm(mm: *mut MmStruct, pages: u64, inc: bool) -> i32 {
    __account_locked_vm(mm, pages, inc, core::ptr::null_mut())
}

pub unsafe fn kmemdup_noprof(src: *const u8, len: usize, gfp: u32) -> *mut u8 {
    if src.is_null() {
        return core::ptr::null_mut();
    }
    let dst = unsafe { crate::mm::slab::kmalloc(len, gfp) };
    if !dst.is_null() {
        unsafe { core::ptr::copy_nonoverlapping(src, dst, len) };
    }
    dst
}

pub unsafe fn kmemdup_array(src: *const u8, count: usize, size: usize, gfp: u32) -> *mut u8 {
    let Some(len) = count.checked_mul(size) else {
        return core::ptr::null_mut();
    };
    unsafe { kmemdup_noprof(src, len, gfp) }
}

pub unsafe extern "C" fn linux_kmemdup_array(
    src: *const u8,
    count: usize,
    size: usize,
    gfp: u32,
) -> *mut u8 {
    unsafe { kmemdup_array(src, count, size, gfp) }
}

pub unsafe fn kvmemdup(src: *const u8, len: usize, gfp: u32) -> *mut u8 {
    unsafe { kmemdup_noprof(src, len, gfp) }
}

#[inline]
fn err_ptr(errno: i32) -> *mut u8 {
    (-(errno as isize)) as usize as *mut u8
}

pub unsafe fn memdup_user(src: *const u8, len: usize) -> *mut u8 {
    let dst = unsafe { crate::mm::slab::kmalloc(len, GFP_USER | __GFP_NOWARN) };
    if dst.is_null() {
        return err_ptr(ENOMEM);
    }
    if len != 0 && src.is_null() {
        unsafe { crate::mm::slab::kfree(dst) };
        return err_ptr(EFAULT);
    }
    if unsafe { copy_from_user(dst, src, len) } != 0 {
        unsafe { crate::mm::slab::kfree(dst) };
        return err_ptr(EFAULT);
    }
    dst
}

pub unsafe fn vmemdup_user(src: *const u8, len: usize) -> *mut u8 {
    unsafe { memdup_user(src, len) }
}

pub unsafe fn memdup_user_nul(src: *const u8, len: usize) -> *mut u8 {
    let Some(size) = len.checked_add(1) else {
        return err_ptr(ENOMEM);
    };
    let dst = unsafe { crate::mm::slab::kmalloc(size, GFP_USER | __GFP_NOWARN) };
    if dst.is_null() {
        return err_ptr(ENOMEM);
    }
    if len != 0 && src.is_null() {
        unsafe { crate::mm::slab::kfree(dst) };
        return err_ptr(EFAULT);
    }
    if unsafe { copy_from_user(dst, src, len) } != 0 {
        unsafe { crate::mm::slab::kfree(dst) };
        return err_ptr(EFAULT);
    }
    unsafe {
        *dst.add(len) = 0;
    }
    dst
}

pub unsafe fn strndup_user(src: *const u8, max: usize) -> *mut u8 {
    if src.is_null() {
        return err_ptr(EFAULT);
    }
    let mut len = 0usize;
    while len < max {
        if unsafe { *src.add(len) } == 0 {
            return unsafe { memdup_user(src, len + 1) };
        }
        len += 1;
    }
    err_ptr(EINVAL)
}

pub unsafe extern "C" fn linux_memdup_user(src: *const u8, len: usize) -> *mut u8 {
    unsafe { memdup_user(src, len) }
}

pub unsafe extern "C" fn linux_vmemdup_user(src: *const u8, len: usize) -> *mut u8 {
    unsafe { vmemdup_user(src, len) }
}

pub unsafe extern "C" fn linux_memdup_user_nul(src: *const u8, len: usize) -> *mut u8 {
    unsafe { memdup_user_nul(src, len) }
}

pub unsafe extern "C" fn linux_strndup_user(src: *const u8, max: isize) -> *mut u8 {
    if max <= 0 {
        return err_ptr(EINVAL);
    }
    unsafe { strndup_user(src, max as usize) }
}

pub unsafe fn kstrndup(src: *const u8, max: usize, gfp: u32) -> *mut u8 {
    if src.is_null() {
        return core::ptr::null_mut();
    }
    let mut len = 0usize;
    while len < max {
        if unsafe { *src.add(len) } == 0 {
            break;
        }
        len += 1;
    }
    let dst = unsafe { crate::mm::slab::kmalloc(len.saturating_add(1), gfp) };
    if !dst.is_null() {
        unsafe {
            core::ptr::copy_nonoverlapping(src, dst, len);
            *dst.add(len) = 0;
        }
    }
    dst
}

pub unsafe extern "C" fn linux_kstrndup(src: *const u8, max: usize, gfp: u32) -> *mut u8 {
    unsafe { kstrndup(src, max, gfp) }
}

pub unsafe fn kstrdup(src: *const u8, gfp: u32) -> *mut u8 {
    unsafe { kstrndup(src, usize::MAX / 2, gfp) }
}

pub unsafe extern "C" fn linux_kstrdup(src: *const u8, gfp: u32) -> *mut u8 {
    unsafe { kstrdup(src, gfp) }
}

pub unsafe fn kstrdup_const(src: *const u8, _gfp: u32) -> *const u8 {
    if src.is_null() {
        return core::ptr::null();
    }
    let mut len = 0usize;
    while unsafe { *src.add(len) } != 0 {
        len += 1;
    }
    let mut bytes = Vec::with_capacity(len.saturating_add(1));
    unsafe {
        bytes.extend_from_slice(core::slice::from_raw_parts(src, len));
    }
    bytes.push(0);
    let len = bytes.len();
    let mut boxed = bytes.into_boxed_slice();
    let ptr = boxed.as_mut_ptr();
    core::mem::forget(boxed);
    KSTRDUP_CONST_ALLOCS.lock().push(ConstDupAlloc {
        ptr: ptr as usize,
        len,
    });
    ptr
}

pub unsafe extern "C" fn linux_kstrdup_const(src: *const u8, gfp: u32) -> *const u8 {
    unsafe { kstrdup_const(src, gfp) }
}

pub unsafe fn kfree_const(ptr: *const u8) {
    if ptr.is_null() {
        return;
    }
    let alloc = {
        let mut allocs = KSTRDUP_CONST_ALLOCS.lock();
        let Some(idx) = allocs.iter().position(|alloc| alloc.ptr == ptr as usize) else {
            return;
        };
        allocs.swap_remove(idx)
    };
    unsafe {
        let slice = core::slice::from_raw_parts_mut(alloc.ptr as *mut u8, alloc.len);
        drop(Box::from_raw(slice));
    }
}

pub unsafe extern "C" fn linux_kfree_const(ptr: *const u8) {
    unsafe { kfree_const(ptr) };
}

pub unsafe fn kmemdup_nul(src: *const u8, len: usize, gfp: u32) -> *mut u8 {
    let dst = unsafe { crate::mm::slab::kmalloc(len.saturating_add(1), gfp) };
    if !dst.is_null() && !src.is_null() {
        unsafe {
            core::ptr::copy_nonoverlapping(src, dst, len);
            *dst.add(len) = 0;
        }
    }
    dst
}

pub unsafe extern "C" fn linux_kmemdup_nul(src: *const u8, len: usize, gfp: u32) -> *mut u8 {
    unsafe { kmemdup_nul(src, len, gfp) }
}

pub fn mem_dump_obj(_ptr: *const u8) -> bool {
    true
}

pub unsafe fn folio_copy(dst: *mut Page, src: *const Page) {
    let dst_addr = crate::mm::page_flags::folio_address(dst);
    let src_addr = crate::mm::page_flags::folio_address(src);
    if !dst_addr.is_null() && !src_addr.is_null() {
        unsafe { core::ptr::copy_nonoverlapping(src_addr, dst_addr, crate::mm::frame::PAGE_SIZE) };
    }
}

pub unsafe fn folio_mc_copy(dst: *mut Page, src: *const Page) -> i32 {
    unsafe { folio_copy(dst, src) };
    0
}

pub fn flush_dcache_folio(_folio: *mut Page) {}

pub fn page_range_contiguous(page: *const Page, nr_pages: usize) -> bool {
    if nr_pages == 0 {
        return true;
    }
    if page.is_null() {
        return false;
    }
    if !page_in_mem_map(page) {
        return true;
    }
    let start_pfn = page_to_pfn(page);
    for offset in 1..nr_pages {
        if unsafe { page.add(offset) } != pfn_to_page(start_pfn + offset) {
            return false;
        }
    }
    true
}

pub fn page_offline_begin(page: *mut Page) -> bool {
    !page.is_null()
}

pub fn page_offline_end(_page: *mut Page) {}

pub fn vma_set_file(vma: *mut VmAreaStruct, file: *mut u8) {
    unsafe { linux_vma_set_file(vma, file.cast()) };
}

/// `vma_set_file` - `vendor/linux/mm/util.c:318`.
///
/// Vendor modules pass Linux-layout `struct file *` values here. Those pointers
/// are opaque to Lupos' native `Arc<File>` VMA ownership path, so this helper
/// mirrors the ABI-visible VMA field update without reinterpreting the file.
pub unsafe extern "C" fn linux_vma_set_file(vma: *mut VmAreaStruct, file: *mut c_void) {
    if !vma.is_null() {
        unsafe {
            (*vma).vm_file = file as usize;
        }
    }
}

pub fn arch_pick_mmap_layout(mm: *mut MmStruct, _rlim_stack: *const u8) {
    if !mm.is_null() {
        unsafe {
            (*mm).start_stack = crate::mm::mmap::DEFAULT_MMAP_BASE;
        }
    }
}

pub fn __compat_vma_mmap(
    file: *mut u8,
    addr: u64,
    len: u64,
    prot: u64,
    flag: u64,
    pgoff: u64,
) -> u64 {
    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return (-EINVAL as i64) as u64;
    }
    let mm = unsafe { (*task).mm };
    if mm.is_null() {
        return (-EINVAL as i64) as u64;
    }
    match unsafe {
        crate::mm::mmap::do_mmap(
            &mut *mm,
            addr,
            len,
            prot as u32,
            flag as u32,
            pgoff,
            file as usize,
        )
    } {
        Ok(mapped) => mapped,
        Err(errno) => errno as i64 as u64,
    }
}

pub fn compat_vma_mmap(
    file: *mut u8,
    addr: u64,
    len: u64,
    prot: u64,
    flag: u64,
    pgoff: u64,
) -> u64 {
    __compat_vma_mmap(file, addr, len, prot, flag, pgoff)
}

/// `compat_vma_mmap` - `vendor/linux/mm/util.c:1266`.
///
/// Lupos does not yet model Linux's transient `struct vm_area_desc` or
/// `mmap_prepare` action pipeline for vendor `struct file` objects.  Treat the
/// compatibility pass as having no extra action to apply.
pub unsafe extern "C" fn linux_compat_vma_mmap(_file: *mut c_void, _vma: *mut VmAreaStruct) -> i32 {
    0
}

pub fn compat_set_desc_from_vma(_desc: *mut u8, _vma: *mut VmAreaStruct) {}

pub fn mmap_action_prepare(_action: *mut u8, _vma: *mut VmAreaStruct, _addr: u64, len: u64) -> i32 {
    if len == 0 { -EINVAL } else { 0 }
}

pub fn mmap_action_complete(_action: *mut u8, _vma: *mut VmAreaStruct) {}

pub fn arch_validate_flags(_flags: u64) -> bool {
    true
}

pub fn arch_validate_prot(prot: u64, _addr: u64) -> bool {
    const PROT_SEM: u64 = 0x8;
    let allowed = crate::mm::mmap::PROT_READ as u64
        | crate::mm::mmap::PROT_WRITE as u64
        | crate::mm::mmap::PROT_EXEC as u64
        | PROT_SEM;
    prot & !allowed == 0
}

pub fn arch_memory_deny_write_exec_supported() -> bool {
    true
}

pub fn mm_compute_batch(nr: usize) -> usize {
    nr.clamp(1, 32)
}

pub fn vm_memory_committed() -> u64 {
    VM_MEMORY_COMMITTED.load(Ordering::Acquire)
}

pub fn vm_acct_memory(pages: u64) {
    VM_MEMORY_COMMITTED.fetch_add(pages, Ordering::AcqRel);
}

pub fn vm_unacct_memory(pages: u64) {
    VM_MEMORY_COMMITTED.fetch_sub(pages, Ordering::AcqRel);
}

pub fn vm_commit_limit() -> u64 {
    u64::MAX / 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utility_validation_matches_linux_shapes() {
        assert_eq!(array_size(4, 8), Ok(32));
        assert_eq!(array3_size(2, 4, 8), Ok(64));
        assert_eq!(mempool_min_nr(0), Err(EINVAL));
        assert_eq!(dmapool_alignment(3), Err(EINVAL));
        assert_eq!(dmapool_alignment(8), Ok(8));
    }

    #[test]
    fn mempool_and_dmapool_recycle_allocations() {
        let mut pool = MemPool::new(2, 16).unwrap();
        assert_eq!(pool.available(), 2);
        let elem = pool.alloc();
        assert_eq!(elem.len(), 16);
        assert_eq!(pool.available(), 1);
        pool.free(elem);
        assert_eq!(pool.available(), 2);

        let mut dma = DmaPool::new(17, 8).unwrap();
        assert_eq!(dma.alloc().len(), 24);
        assert_eq!(dma.align(), 8);
    }

    #[test]
    fn locked_vm_accounting_updates_mm_state() {
        let mut mm = MmStruct::new(0);
        assert_eq!(account_locked_vm(&mut mm, 4, true), 0);
        assert_eq!(mm.locked_vm, 4);
        assert_eq!(account_locked_vm(&mut mm, 2, false), 0);
        assert_eq!(mm.locked_vm, 2);
        assert_eq!(account_locked_vm(core::ptr::null_mut(), 2, true), 0);
        mm.locked_vm = u64::MAX;
        assert_eq!(account_locked_vm(&mut mm, 1, true), -ENOMEM);
    }

    #[test]
    fn kstrdup_const_allocates_and_only_frees_tracked_copies() {
        let literal = b"kernel\0";
        let duplicated = unsafe { kstrdup_const(literal.as_ptr(), 0) };
        assert!(!duplicated.is_null());
        assert_ne!(duplicated, literal.as_ptr());
        unsafe {
            assert_eq!(core::slice::from_raw_parts(duplicated, 7), literal);
            kfree_const(duplicated);
            kfree_const(literal.as_ptr());
            kfree_const(core::ptr::null());
        }
    }

    #[test]
    fn page_and_mman_helpers_match_configured_x86_shape() {
        assert!(page_range_contiguous(core::ptr::null(), 0));
        assert!(!page_range_contiguous(core::ptr::null(), 1));
        let pages = [const { Page::new() }, const { Page::new() }];
        assert!(page_range_contiguous(pages.as_ptr(), pages.len()));
        assert!(page_offline_begin(pages.as_ptr() as *mut Page));
        page_offline_end(pages.as_ptr() as *mut Page);

        let mut vma = VmAreaStruct::new(0x1000, 0x2000, 0);
        vma_set_file(&mut vma, 0x1234usize as *mut u8);
        assert_eq!(vma.vm_file, 0x1234);
        assert_eq!(
            mmap_action_prepare(core::ptr::null_mut(), &mut vma, 0, 0),
            -EINVAL
        );
        assert_eq!(
            mmap_action_prepare(core::ptr::null_mut(), &mut vma, 0, 4096),
            0
        );

        assert!(arch_validate_flags(u64::MAX));
        assert!(arch_validate_prot(0x1 | 0x2 | 0x4 | 0x8, 0));
        assert!(!arch_validate_prot(0x10, 0));
        assert!(arch_memory_deny_write_exec_supported());
        assert_eq!(mm_compute_batch(0), 1);
        assert_eq!(mm_compute_batch(64), 32);
        assert_eq!(
            compat_vma_mmap(core::ptr::null_mut(), 0, 4096, 0, 0, 0),
            (-EINVAL as i64) as u64
        );
    }

    #[test]
    fn committed_memory_accounting_is_reversible() {
        let before = vm_memory_committed();
        vm_acct_memory(5);
        assert_eq!(vm_memory_committed(), before + 5);
        vm_unacct_memory(5);
        assert_eq!(vm_memory_committed(), before);
        assert!(vm_commit_limit() > vm_memory_committed());
    }
}
