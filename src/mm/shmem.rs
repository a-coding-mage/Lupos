//! linux-parity: partial
//! linux-source: vendor/linux/mm/shmem.c
//! linux-source: vendor/linux/drivers/base/devtmpfs.c
//! test-origin: linux:vendor/linux/mm/shmem.c
//! shmem, memfd, secretmem, and userfaultfd.
//!
//! Implements the memory-owned pieces from:
//! - `vendor/linux/mm/memfd.c`
//! - `vendor/linux/mm/memfd_luo.c`
//! - `vendor/linux/mm/secretmem.c`
//! - `vendor/linux/mm/shmem.c`
//! - `vendor/linux/mm/shmem_quota.c`
//! - `vendor/linux/mm/userfaultfd.c`
//!
//! The VFS owns file descriptors and path lookup; this module owns the
//! page-cache-like shmem object, seals, quotas, secret mappings, and
//! userfaultfd registration state.

extern crate alloc;

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec;
use alloc::vec::Vec;
use core::ffi::c_void;

use spin::Mutex;

use crate::include::uapi::errno::{EINVAL, ENODEV, ENOMEM, EPERM};
use crate::kernel::module::{export_symbol, find_symbol};
use crate::mm::address_space::AddressSpace;
use crate::mm::frame::PAGE_SIZE;
use crate::mm::mm_types::VmAreaStruct;
use crate::mm::page::Page;
use crate::mm::page_flags::{GFP_KERNEL, GfpFlags};

pub const MFD_CLOEXEC: u32 = 0x0001;
pub const MFD_ALLOW_SEALING: u32 = 0x0002;
pub const MFD_HUGETLB: u32 = 0x0004;
pub const MFD_NOEXEC_SEAL: u32 = 0x0008;
pub const MFD_EXEC: u32 = 0x0010;
pub const MFD_HUGE_SHIFT: u32 = 26;
pub const MFD_HUGE_MASK: u32 = 0x3f;
pub const UFFD_CLOEXEC: i32 = crate::include::uapi::fcntl::O_CLOEXEC as i32;
pub const UFFD_NONBLOCK: i32 = 0x800;
pub const UFFD_USER_MODE_ONLY: i32 = 1;

pub const F_SEAL_SEAL: u32 = 0x0001;
pub const F_SEAL_SHRINK: u32 = 0x0002;
pub const F_SEAL_GROW: u32 = 0x0004;
pub const F_SEAL_WRITE: u32 = 0x0008;
pub const F_SEAL_FUTURE_WRITE: u32 = 0x0010;

const LINUX_INODE_I_MAPPING_OFFSET: usize = 48;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("shmem_file_setup", linux_shmem_file_setup as usize, true);
    export_symbol_once(
        "shmem_read_folio_gfp",
        linux_shmem_read_folio_gfp as usize,
        true,
    );
    export_symbol_once(
        "shmem_read_mapping_page_gfp",
        linux_shmem_read_mapping_page_gfp as usize,
        true,
    );
    export_symbol_once(
        "shmem_truncate_range",
        linux_shmem_truncate_range as usize,
        true,
    );
    export_symbol_once("shmem_writeout", linux_shmem_writeout as usize, true);
}
pub const F_SEAL_EXEC: u32 = 0x0020;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ShmemObject {
    len: usize,
    pages: BTreeMap<usize, Vec<u8>>,
    hwpoison_pages: BTreeSet<usize>,
    seals: u32,
    quota_pages: Option<usize>,
}

impl ShmemObject {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_quota_pages(quota_pages: usize) -> Self {
        Self {
            len: 0,
            pages: BTreeMap::new(),
            hwpoison_pages: BTreeSet::new(),
            seals: 0,
            quota_pages: Some(quota_pages),
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn resize(&mut self, len: usize) -> Result<(), i32> {
        if len < self.len && self.seals & F_SEAL_SHRINK != 0 {
            return Err(EPERM);
        }
        if len > self.len && self.seals & F_SEAL_GROW != 0 {
            return Err(EPERM);
        }
        if let Some(quota) = self.quota_pages {
            if len.div_ceil(PAGE_SIZE) > quota {
                return Err(ENOMEM);
            }
        }
        if len < self.len {
            let keep_pages = len.div_ceil(PAGE_SIZE);
            self.pages.retain(|page_idx, _| *page_idx < keep_pages);
            self.hwpoison_pages
                .retain(|page_idx| *page_idx < keep_pages);
            if len % PAGE_SIZE != 0
                && let Some(page) = self.pages.get_mut(&(len / PAGE_SIZE))
            {
                page[len % PAGE_SIZE..].fill(0);
            }
        }
        self.len = len;
        Ok(())
    }

    pub fn zero_range(&mut self, offset: usize, len: usize, keep_size: bool) -> Result<(), i32> {
        if self.seals & (F_SEAL_WRITE | F_SEAL_FUTURE_WRITE) != 0 {
            return Err(EPERM);
        }
        let end = offset.checked_add(len).ok_or(EINVAL)?;
        if !keep_size && end > self.len {
            self.resize(end)?;
        }
        let zero_end = end.min(self.len);
        if offset >= zero_end {
            return Ok(());
        }
        let mut pos = offset;
        while pos < zero_end {
            let page_idx = pos / PAGE_SIZE;
            let page_off = pos % PAGE_SIZE;
            let chunk = (PAGE_SIZE - page_off).min(zero_end - pos);
            if let Some(page) = self.pages.get_mut(&page_idx) {
                page[page_off..page_off + chunk].fill(0);
                if page.iter().all(|byte| *byte == 0) {
                    self.pages.remove(&page_idx);
                }
            }
            pos += chunk;
        }
        Ok(())
    }

    pub fn hwpoison_range(&mut self, offset: usize, len: usize) -> Result<(), i32> {
        let end = offset.checked_add(len).ok_or(EINVAL)?;
        if len == 0 || offset >= self.len {
            return Ok(());
        }
        let poison_end = end.min(self.len);
        let first = offset / PAGE_SIZE;
        let last = poison_end.saturating_sub(1) / PAGE_SIZE;
        for page_idx in first..=last {
            self.hwpoison_pages.insert(page_idx);
        }
        Ok(())
    }

    pub fn first_hwpoison_offset(&self, offset: usize, len: usize) -> Option<usize> {
        let end = offset.checked_add(len)?.min(self.len);
        if len == 0 || offset >= end {
            return None;
        }
        let first = offset / PAGE_SIZE;
        let last = end.saturating_sub(1) / PAGE_SIZE;
        self.hwpoison_pages.range(first..=last).next().map(|page| {
            let page_start = page.saturating_mul(PAGE_SIZE);
            page_start.max(offset)
        })
    }

    pub fn write_at(&mut self, offset: usize, data: &[u8]) -> Result<usize, i32> {
        if self.seals & (F_SEAL_WRITE | F_SEAL_FUTURE_WRITE) != 0 {
            return Err(EPERM);
        }
        let end = offset.checked_add(data.len()).ok_or(EINVAL)?;
        if end > self.len {
            self.resize(end)?;
        }
        let mut copied = 0usize;
        while copied < data.len() {
            let pos = offset + copied;
            let page_idx = pos / PAGE_SIZE;
            let page_off = pos % PAGE_SIZE;
            let chunk = (PAGE_SIZE - page_off).min(data.len() - copied);
            let page = self
                .pages
                .entry(page_idx)
                .or_insert_with(|| vec![0u8; PAGE_SIZE]);
            page[page_off..page_off + chunk].copy_from_slice(&data[copied..copied + chunk]);
            copied += chunk;
        }
        Ok(data.len())
    }

    pub fn read_at(&self, offset: usize, out: &mut [u8]) -> usize {
        if offset >= self.len {
            return 0;
        }
        let end = core::cmp::min(offset + out.len(), self.len);
        let len = end - offset;
        out[..len].fill(0);
        let mut copied = 0usize;
        while copied < len {
            let pos = offset + copied;
            let page_idx = pos / PAGE_SIZE;
            let page_off = pos % PAGE_SIZE;
            let chunk = (PAGE_SIZE - page_off).min(len - copied);
            if let Some(page) = self.pages.get(&page_idx) {
                out[copied..copied + chunk].copy_from_slice(&page[page_off..page_off + chunk]);
            }
            copied += chunk;
        }
        len
    }

    pub fn add_seals(&mut self, seals: u32) -> Result<(), i32> {
        let valid = F_SEAL_SEAL
            | F_SEAL_SHRINK
            | F_SEAL_GROW
            | F_SEAL_WRITE
            | F_SEAL_FUTURE_WRITE
            | F_SEAL_EXEC;
        if seals & !valid != 0 {
            return Err(EINVAL);
        }
        if self.seals & F_SEAL_SEAL != 0 {
            return Err(EPERM);
        }
        self.seals |= seals;
        Ok(())
    }

    pub fn seals(&self) -> u32 {
        self.seals
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SecretMem {
    pub id: u64,
    pub len: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UserfaultRegistration {
    pub start: u64,
    pub len: u64,
    pub missing: bool,
    pub write_protect: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemfdLuoImage {
    pub source_id: u64,
    pub len: usize,
    pub pos: usize,
    pub nr_folios: usize,
}

struct ShmemState {
    next_id: u64,
    memfds: BTreeMap<u64, ShmemObject>,
    memfd_flags: BTreeMap<u64, u32>,
    secretmem: BTreeMap<u64, SecretMem>,
    userfaultfd: Vec<UserfaultRegistration>,
    luo_images: Vec<MemfdLuoImage>,
}

impl ShmemState {
    const fn new() -> Self {
        Self {
            next_id: 1,
            memfds: BTreeMap::new(),
            memfd_flags: BTreeMap::new(),
            secretmem: BTreeMap::new(),
            userfaultfd: Vec::new(),
            luo_images: Vec::new(),
        }
    }

    fn reset(&mut self) {
        self.next_id = 1;
        self.memfds.clear();
        self.memfd_flags.clear();
        self.secretmem.clear();
        self.userfaultfd.clear();
        self.luo_images.clear();
    }
}

static SHMEM_STATE: Mutex<ShmemState> = Mutex::new(ShmemState::new());

pub fn validate_memfd_flags(flags: u32) -> Result<(), i32> {
    const MFD_ALL_FLAGS: u32 =
        MFD_CLOEXEC | MFD_ALLOW_SEALING | MFD_HUGETLB | MFD_NOEXEC_SEAL | MFD_EXEC;
    const MFD_HUGE_FLAGS: u32 = MFD_HUGE_MASK << MFD_HUGE_SHIFT;

    let allowed = if flags & MFD_HUGETLB != 0 {
        MFD_ALL_FLAGS | MFD_HUGE_FLAGS
    } else {
        MFD_ALL_FLAGS
    };
    if flags & !allowed != 0 || flags & MFD_EXEC != 0 && flags & MFD_NOEXEC_SEAL != 0 {
        Err(EINVAL)
    } else {
        Ok(())
    }
}

pub fn create_memfd(flags: u32) -> Result<u64, i32> {
    validate_memfd_flags(flags)?;
    if flags & MFD_HUGETLB != 0 {
        let shift = (flags >> MFD_HUGE_SHIFT) & MFD_HUGE_MASK;
        if shift != 0 && shift != crate::arch::x86::mm::paging::PMD_SHIFT {
            return Err(ENODEV);
        }
    }
    let mut state = SHMEM_STATE.lock();
    let id = state.next_id;
    state.next_id += 1;
    let mut obj = ShmemObject::new();
    if flags & MFD_ALLOW_SEALING == 0 {
        obj.seals |= F_SEAL_SEAL;
    }
    if flags & MFD_NOEXEC_SEAL != 0 {
        obj.seals &= !F_SEAL_SEAL;
        obj.seals |= F_SEAL_EXEC;
    }
    state.memfds.insert(id, obj);
    state.memfd_flags.insert(id, flags);
    Ok(id)
}

pub fn memfd_object(id: u64) -> Option<ShmemObject> {
    SHMEM_STATE.lock().memfds.get(&id).cloned()
}

pub fn with_memfd_mut<R>(id: u64, f: impl FnOnce(&mut ShmemObject) -> R) -> Option<R> {
    SHMEM_STATE.lock().memfds.get_mut(&id).map(f)
}

pub fn memfd_luo_preserve(id: u64, pos: usize) -> Result<MemfdLuoImage, i32> {
    let mut state = SHMEM_STATE.lock();
    let flags = *state.memfd_flags.get(&id).ok_or(EINVAL)?;
    if flags & MFD_HUGETLB != 0 {
        return Err(EINVAL);
    }
    let object = state.memfds.get(&id).ok_or(EINVAL)?;
    let image = MemfdLuoImage {
        source_id: id,
        len: object.len(),
        pos,
        nr_folios: object.len().div_ceil(PAGE_SIZE),
    };
    state.luo_images.push(image);
    Ok(image)
}

pub fn memfd_luo_restore(image: MemfdLuoImage) -> Result<u64, i32> {
    let id = create_memfd(MFD_ALLOW_SEALING)?;
    with_memfd_mut(id, |obj| obj.resize(image.len)).ok_or(EINVAL)??;
    Ok(id)
}

pub fn memfd_luo_images() -> usize {
    SHMEM_STATE.lock().luo_images.len()
}

pub fn validate_userfaultfd_flags(flags: i32) -> Result<(), i32> {
    if flags & !(UFFD_CLOEXEC | UFFD_NONBLOCK | UFFD_USER_MODE_ONLY) != 0 {
        Err(EINVAL)
    } else {
        Ok(())
    }
}

pub fn userfaultfd_register(
    start: u64,
    len: u64,
    missing: bool,
    write_protect: bool,
) -> Result<(), i32> {
    if len == 0 || start % PAGE_SIZE as u64 != 0 {
        return Err(EINVAL);
    }
    SHMEM_STATE.lock().userfaultfd.push(UserfaultRegistration {
        start,
        len,
        missing,
        write_protect,
    });
    Ok(())
}

fn uffd_ranges_overlap(a_start: u64, a_len: u64, b_start: u64, b_len: u64) -> bool {
    let Some(a_end) = a_start.checked_add(a_len) else {
        return true;
    };
    let Some(b_end) = b_start.checked_add(b_len) else {
        return true;
    };
    a_start < b_end && a_end > b_start
}

pub fn userfaultfd_unregister_range(start: u64, len: u64) {
    SHMEM_STATE
        .lock()
        .userfaultfd
        .retain(|reg| !uffd_ranges_overlap(reg.start, reg.len, start, len));
}

pub fn userfaultfd_range_registered(start: u64, len: u64) -> bool {
    SHMEM_STATE.lock().userfaultfd.iter().any(|reg| {
        (reg.missing || reg.write_protect) && uffd_ranges_overlap(reg.start, reg.len, start, len)
    })
}

pub fn userfaultfd_fault(addr: u64, write: bool) -> bool {
    SHMEM_STATE.lock().userfaultfd.iter().any(|reg| {
        addr >= reg.start
            && addr < reg.start + reg.len
            && (reg.missing || (write && reg.write_protect))
    })
}

pub fn memfd_secret(flags: u32) -> i64 {
    match create_secretmem(flags, 0) {
        Ok(id) => id as i64,
        Err(errno) => -(errno as i64),
    }
}

pub fn create_secretmem(flags: u32, len: usize) -> Result<u64, i32> {
    if flags != 0 {
        return Err(EINVAL);
    }
    let mut state = SHMEM_STATE.lock();
    let id = state.next_id;
    state.next_id += 1;
    state.secretmem.insert(id, SecretMem { id, len });
    Ok(id)
}

pub fn secretmem_object(id: u64) -> Option<SecretMem> {
    SHMEM_STATE.lock().secretmem.get(&id).copied()
}

pub fn with_secretmem_mut<R>(id: u64, f: impl FnOnce(&mut SecretMem) -> R) -> Option<R> {
    SHMEM_STATE.lock().secretmem.get_mut(&id).map(f)
}

pub fn shmem_quota_enabled() -> bool {
    true
}

// ---------------------------------------------------------------------------
// Linux-visible shmem_fs.h wrappers
// ---------------------------------------------------------------------------

pub fn SHMEM_I(inode: *mut u8) -> *mut u8 {
    inode
}

pub fn shmem_file(file: *const u8) -> bool {
    if file.is_null() {
        return false;
    }
    SHMEM_STATE
        .lock()
        .memfds
        .contains_key(&(file as usize as u64))
}

pub fn shmem_mapping(mapping: *const AddressSpace) -> bool {
    !mapping.is_null()
}

pub fn shmem_file_setup(_name: *const u8, size: u64, flags: u64) -> *mut u8 {
    if validate_memfd_flags(flags as u32).is_err() {
        return core::ptr::null_mut();
    }
    match create_memfd(flags as u32) {
        Ok(id) => {
            let _ = with_memfd_mut(id, |obj| obj.resize(size as usize));
            id as usize as *mut u8
        }
        Err(_) => core::ptr::null_mut(),
    }
}

pub fn shmem_file_setup_with_mnt(_mnt: *mut u8, name: *const u8, size: u64, flags: u64) -> *mut u8 {
    shmem_file_setup(name, size, flags)
}

pub fn shmem_kernel_file_setup(name: *const u8, size: u64, flags: u64) -> *mut u8 {
    shmem_file_setup(name, size, flags)
}

pub fn shmem_zero_setup(vma: *mut VmAreaStruct) -> i32 {
    if vma.is_null() {
        return -EINVAL;
    }
    let file = shmem_file_setup(core::ptr::null(), unsafe { (*vma).size() }, 0);
    if file.is_null() {
        return -ENOMEM;
    }
    unsafe {
        (*vma).vm_file = file as usize;
    }
    0
}

pub fn shmem_zero_setup_desc(vma: *mut VmAreaStruct, _desc: *mut u8) -> i32 {
    shmem_zero_setup(vma)
}

pub fn shmem_get_unmapped_area(
    _file: *mut u8,
    addr: u64,
    _len: u64,
    _pgoff: u64,
    _flags: u64,
) -> u64 {
    addr
}

pub fn shmem_hpage_pmd_enabled(_vma: *mut u8, _addr: u64, _shmem_huge_force: bool) -> bool {
    crate::mm::huge::transparent_hugepage_enabled()
}

pub fn shmem_allowable_huge_orders(
    _inode: *mut u8,
    _vma: *mut u8,
    _addr: u64,
    _global: bool,
) -> u64 {
    1u64 << crate::mm::huge::HPAGE_PMD_ORDER
}

pub fn shmem_fallocend(size: u64) -> u64 {
    size
}

pub fn shmem_freeze(_mapping: *mut AddressSpace) -> bool {
    false
}

pub fn shmem_lock(_file: *mut u8, _lock: bool, _user: *mut u8) -> i32 {
    0
}

pub fn shmem_charge(_inode: *mut u8, _pages: usize) -> i32 {
    0
}

pub fn shmem_uncharge(_inode: *mut u8, _pages: usize) {}

pub fn shmem_partial_swap_usage(_mapping: *mut AddressSpace, _start: u64, _end: u64) -> u64 {
    0
}

pub fn shmem_swap_usage(_mapping: *mut AddressSpace) -> u64 {
    0
}

pub fn shmem_unuse(_swap: u32, _page: *mut Page) -> i32 {
    0
}

pub fn shmem_unlock_mapping(_mapping: *mut AddressSpace) {}

pub fn shmem_truncate_mapping_range(mapping: *mut AddressSpace, start: u64, end: u64) {
    unsafe { crate::mm::filemap::truncate_inode_pages_range(mapping, start, end) };
}

pub fn shmem_get_folio(
    mapping: *mut AddressSpace,
    index: u64,
    folio: *mut *mut Page,
    gfp: GfpFlags,
) -> i32 {
    if folio.is_null() {
        return -EINVAL;
    }
    let page = unsafe { crate::mm::filemap::__filemap_get_folio(mapping, index, 1, gfp) };
    unsafe {
        *folio = page;
    }
    if page.is_null() { -ENOMEM } else { 0 }
}

pub fn shmem_read_folio_into_gfp(
    mapping: *mut AddressSpace,
    folio: *mut Page,
    gfp: GfpFlags,
) -> i32 {
    unsafe { crate::mm::filemap::mapping_read_folio_gfp(mapping, folio, gfp) }
}

pub fn shmem_read_folio(mapping: *mut AddressSpace, folio: *mut Page) -> i32 {
    shmem_read_folio_into_gfp(mapping, folio, GFP_KERNEL)
}

pub fn shmem_read_mapping_page_gfp(
    mapping: *mut AddressSpace,
    index: u64,
    gfp: GfpFlags,
) -> *mut Page {
    unsafe { crate::mm::filemap::read_cache_page_gfp(mapping, index, gfp) }
}

pub fn shmem_read_mapping_page(mapping: *mut AddressSpace, index: u64) -> *mut Page {
    shmem_read_mapping_page_gfp(mapping, index, crate::mm::page_flags::GFP_KERNEL)
}

pub fn shmem_writeout_folio(_folio: *mut Page, _wbc: *mut u8) -> i32 {
    -EINVAL
}

pub unsafe extern "C" fn linux_shmem_file_setup(
    name: *const u8,
    size: i64,
    flags: u64,
) -> *mut c_void {
    shmem_file_setup(name, size.max(0) as u64, flags).cast()
}

/// `shmem_read_folio_gfp` - `vendor/linux/mm/shmem.c:5922`.
pub unsafe extern "C" fn linux_shmem_read_folio_gfp(
    mapping: *mut AddressSpace,
    index: u64,
    gfp: GfpFlags,
) -> *mut Page {
    shmem_read_mapping_page_gfp(mapping, index, gfp)
}

pub unsafe extern "C" fn linux_shmem_read_mapping_page_gfp(
    mapping: *mut AddressSpace,
    index: u64,
    gfp: GfpFlags,
) -> *mut Page {
    shmem_read_mapping_page_gfp(mapping, index, gfp)
}

pub unsafe extern "C" fn linux_shmem_truncate_range(inode: *mut c_void, start: i64, end: u64) {
    if inode.is_null() {
        return;
    }
    let mapping = unsafe {
        *inode
            .cast::<u8>()
            .add(LINUX_INODE_I_MAPPING_OFFSET)
            .cast::<*mut AddressSpace>()
    };
    shmem_truncate_mapping_range(mapping, start.max(0) as u64, end);
}

pub unsafe extern "C" fn linux_shmem_writeout(
    folio: *mut Page,
    _plug: *mut *mut c_void,
    _folio_list: *mut c_void,
) -> i32 {
    shmem_writeout_folio(folio, core::ptr::null_mut())
}

pub fn shmem_init() -> i32 {
    0
}

pub fn shmem_init_fs_context(_fc: *mut u8) -> i32 {
    0
}

#[cfg(test)]
pub fn reset_for_tests() {
    SHMEM_STATE.lock().reset();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK;

    #[test]
    fn shmem_object_reads_writes_resizes_and_honors_seals() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();
        let mut obj = ShmemObject::with_quota_pages(1);
        assert_eq!(obj.write_at(0, b"lupos"), Ok(5));
        let mut out = [0u8; 5];
        assert_eq!(obj.read_at(0, &mut out), 5);
        assert_eq!(&out, b"lupos");
        assert_eq!(obj.zero_range(1, 3, true), Ok(()));
        assert_eq!(obj.read_at(0, &mut out), 5);
        assert_eq!(&out, b"l\0\0\0s");
        assert_eq!(obj.resize(PAGE_SIZE + 1), Err(ENOMEM));
        assert_eq!(obj.add_seals(F_SEAL_WRITE), Ok(()));
        assert_eq!(obj.write_at(0, b"x"), Err(EPERM));
        assert_eq!(obj.zero_range(0, 1, true), Err(EPERM));
    }

    #[test]
    fn shmem_resize_is_sparse_and_reads_holes_as_zeroes() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();
        let mut obj = ShmemObject::new();
        let large_len = 260_046_848;

        assert_eq!(obj.resize(large_len), Ok(()));
        assert_eq!(obj.len(), large_len);
        assert!(obj.pages.is_empty());

        let offset = large_len - 3;
        assert_eq!(obj.write_at(offset, b"end"), Ok(3));
        let mut out = [0xffu8; 8];
        assert_eq!(obj.read_at(offset - 5, &mut out), 8);
        assert_eq!(&out, b"\0\0\0\0\0end");

        assert_eq!(obj.zero_range(offset, 3, true), Ok(()));
        assert!(obj.pages.is_empty());
        assert_eq!(obj.resize(PAGE_SIZE), Ok(()));
        assert_eq!(obj.len(), PAGE_SIZE);
    }

    #[test]
    fn memfd_secretmem_and_userfaultfd_are_stateful() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();
        let id = create_memfd(MFD_ALLOW_SEALING).unwrap();
        with_memfd_mut(id, |obj| obj.write_at(0, b"abc"))
            .unwrap()
            .unwrap();
        assert_eq!(memfd_object(id).unwrap().len(), 3);

        let secret = create_secretmem(0, 4096).unwrap();
        assert_eq!(secretmem_object(secret).unwrap().len, 4096);
        assert!(memfd_secret(0) > 0);
        assert_eq!(memfd_secret(1), -(EINVAL as i64));

        assert_eq!(userfaultfd_register(0x4000, 0x1000, true, false), Ok(()));
        assert!(userfaultfd_fault(0x4000, false));
        assert_eq!(
            validate_userfaultfd_flags(UFFD_USER_MODE_ONLY | UFFD_CLOEXEC | UFFD_NONBLOCK),
            Ok(())
        );
        assert!(shmem_quota_enabled());
    }

    #[test]
    fn memfd_luo_preserves_size_position_and_rejects_hugetlb() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();

        let id = create_memfd(MFD_ALLOW_SEALING).unwrap();
        with_memfd_mut(id, |obj| obj.write_at(PAGE_SIZE + 4, b"x"))
            .unwrap()
            .unwrap();
        let image = memfd_luo_preserve(id, 17).unwrap();
        assert_eq!(image.len, PAGE_SIZE + 5);
        assert_eq!(image.pos, 17);
        assert_eq!(image.nr_folios, 2);
        assert_eq!(memfd_luo_images(), 1);

        let restored = memfd_luo_restore(image).unwrap();
        assert_eq!(memfd_object(restored).unwrap().len(), PAGE_SIZE + 5);

        let huge = create_memfd(MFD_ALLOW_SEALING | MFD_HUGETLB).unwrap();
        assert_eq!(memfd_luo_preserve(huge, 0), Err(EINVAL));
    }

    #[test]
    fn linux_visible_shmem_wrappers_validate_objects_and_zero_setup() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();

        assert_eq!(SHMEM_I(0x1234usize as *mut u8) as usize, 0x1234);
        assert!(!shmem_file(core::ptr::null()));
        let file = shmem_file_setup(
            core::ptr::null(),
            PAGE_SIZE as u64,
            MFD_ALLOW_SEALING as u64,
        );
        assert!(!file.is_null());
        assert!(shmem_file(file));

        let mapping = AddressSpace::new();
        assert!(shmem_mapping(&mapping));
        assert!(!shmem_mapping(core::ptr::null()));
        assert_eq!(shmem_fallocend(17), 17);
        assert_eq!(shmem_init(), 0);
        assert_eq!(shmem_init_fs_context(core::ptr::null_mut()), 0);

        let mut vma = VmAreaStruct::new(0x1000, 0x3000, 0);
        assert_eq!(shmem_zero_setup(&raw mut vma), 0);
        assert_ne!(vma.vm_file, 0);
        assert!(shmem_file(vma.vm_file as *const u8));
        assert_eq!(shmem_zero_setup(core::ptr::null_mut()), -EINVAL);
    }
}

// ---------------------------------------------------------------------------
// tmpfs/devtmpfs/devpts VFS mount glue moved under mm/shmem ownership
// ---------------------------------------------------------------------------

pub mod tmpfs_vfs {
    //! tmpfs (M42).
    //!
    //! Mirrors `vendor/linux/mm/shmem.c`.  The minimal lupos tmpfs backs file
    //! contents with the same `RamBytes` payload as ramfs — full page-cache /
    //! shmem integration with swap-out lands when M16 swap finishes plumbing.

    use alloc::string::String;
    use core::sync::atomic::Ordering;
    use spin::Mutex;

    use crate::fs::dcache::{d_alloc, d_drop, d_lookup};
    use crate::fs::libfs::{
        empty_ram_bytes, empty_ram_dir, ram_file_read, ram_file_write, simple_lookup,
        simple_readdir, simple_rmdir, simple_unlink,
    };
    use crate::fs::ops::{FileOps, InodeOps, SuperOps};
    use crate::fs::super_block::{FileSystemType, register_filesystem};
    use crate::fs::types::{
        DentryRef, Inode, InodeKind, InodePrivate, InodeRef, SuperBlock, SuperBlockRef,
        init_inode_metadata, init_inode_owner, touch_inode_now,
    };
    use crate::include::uapi::errno::{EEXIST, EINVAL, EPERM};
    use crate::include::uapi::stat::S_IFDIR;
    use crate::linux_driver_abi::input::evdev_chardev::EVDEV_FILE_OPS;
    use crate::linux_driver_abi::tty::pty::PTMX_FILE_OPS;
    use crate::linux_driver_abi::video::fbdev::FBDEV_FILE_OPS;

    const TMPFS_MAGIC: u64 = 0x01021994;

    // Linux keeps one internal devtmpfs mount and device_add() publishes into
    // it regardless of when userspace mounts devtmpfs.  Lupos' VFS exposes the
    // mounted root directly, so retain that root for the equivalent block
    // device registration callback.
    static DEVTMPFS_ROOT: Mutex<Option<DentryRef>> = Mutex::new(None);
    static DEVTMPFS_SUPERBLOCK: Mutex<Option<SuperBlockRef>> = Mutex::new(None);
    const DEVTMPFS_BLOCK_INODE_MARKER: usize = 0x6465_7674_6d70_626c;
    const BOGO_DIRENT_SIZE: u64 = 20;

    pub static TMPFS_DIR_INODE_OPS: InodeOps = InodeOps {
        name: "tmpfs_dir",
        lookup: Some(simple_lookup),
        create: Some(tmpfs_create),
        mkdir: Some(tmpfs_mkdir),
        unlink: Some(simple_unlink),
        rmdir: Some(simple_rmdir),
        rename: None,
        symlink: Some(tmpfs_symlink),
        readlink: None,
    };
    pub static TMPFS_FILE_INODE_OPS: InodeOps = InodeOps {
        name: "tmpfs_file",
        lookup: None,
        create: None,
        mkdir: None,
        unlink: None,
        rmdir: None,
        rename: None,
        symlink: None,
        readlink: None,
    };
    pub static TMPFS_SYMLINK_INODE_OPS: InodeOps = InodeOps {
        name: "tmpfs_symlink",
        lookup: None,
        create: None,
        mkdir: None,
        unlink: None,
        rmdir: None,
        rename: None,
        symlink: None,
        readlink: Some(tmpfs_readlink),
    };

    pub static TMPFS_FILE_OPS: FileOps = FileOps {
        name: "tmpfs_file",
        read: Some(ram_file_read),
        write: Some(ram_file_write),
        llseek: None,
        fsync: Some(|_| Ok(())),
        poll: None,
        ioctl: None,
        mmap: None,
        release: None,
        readdir: None,
    };
    pub static TMPFS_DIR_FILE_OPS: FileOps = FileOps {
        name: "tmpfs_dir",
        read: None,
        write: None,
        llseek: None,
        fsync: Some(|_| Ok(())),
        poll: None,
        ioctl: None,
        mmap: None,
        release: None,
        readdir: Some(simple_readdir),
    };
    pub static TMPFS_SYMLINK_FILE_OPS: FileOps = FileOps {
        name: "tmpfs_symlink",
        read: None,
        write: None,
        llseek: None,
        fsync: None,
        poll: None,
        ioctl: None,
        mmap: None,
        release: None,
        readdir: None,
    };

    pub static TMPFS_SUPER_OPS: SuperOps = SuperOps {
        name: "tmpfs",
        statfs: None,
        put_super: None,
        sync_fs: None,
        alloc_inode: None,
        destroy_inode: None,
    };

    fn make_dir(sb: &SuperBlockRef, dir: Option<&InodeRef>, mode: u32) -> InodeRef {
        let i = Inode::new(
            sb.alloc_ino(),
            InodeKind::Directory,
            mode | S_IFDIR,
            &TMPFS_DIR_INODE_OPS,
            &TMPFS_DIR_FILE_OPS,
            empty_ram_dir(),
        );
        init_inode_owner(&i, dir, mode | S_IFDIR);
        init_inode_metadata(
            &i,
            i.uid.load(Ordering::Acquire),
            i.gid.load(Ordering::Acquire),
            2,
            0,
        );
        i.size.store(2 * BOGO_DIRENT_SIZE, Ordering::Release);
        *i.sb.lock() = Some(sb.clone());
        i
    }
    fn make_reg(sb: &SuperBlockRef, dir: Option<&InodeRef>, mode: u32) -> InodeRef {
        let i = Inode::new(
            sb.alloc_ino(),
            InodeKind::Regular,
            mode,
            &TMPFS_FILE_INODE_OPS,
            &TMPFS_FILE_OPS,
            empty_ram_bytes(),
        );
        init_inode_owner(&i, dir, mode);
        init_inode_metadata(
            &i,
            i.uid.load(Ordering::Acquire),
            i.gid.load(Ordering::Acquire),
            1,
            0,
        );
        *i.sb.lock() = Some(sb.clone());
        i
    }
    fn make_symlink(
        sb: &SuperBlockRef,
        dir: Option<&InodeRef>,
        mode: u32,
        target: &str,
    ) -> InodeRef {
        let i = Inode::new(
            sb.alloc_ino(),
            InodeKind::Symlink,
            mode,
            &TMPFS_SYMLINK_INODE_OPS,
            &TMPFS_SYMLINK_FILE_OPS,
            empty_ram_bytes(),
        );
        init_inode_owner(&i, dir, mode);
        init_inode_metadata(
            &i,
            i.uid.load(Ordering::Acquire),
            i.gid.load(Ordering::Acquire),
            1,
            0,
        );
        if let InodePrivate::RamBytes(bytes) = &i.private {
            bytes.lock().extend_from_slice(target.as_bytes());
        }
        i.size.store(target.len() as u64, Ordering::Release);
        *i.sb.lock() = Some(sb.clone());
        i
    }

    fn make_special(
        sb: &SuperBlockRef,
        kind: InodeKind,
        mode: u32,
        fops: &'static FileOps,
    ) -> InodeRef {
        let i = Inode::new(
            sb.alloc_ino(),
            kind,
            mode,
            &TMPFS_FILE_INODE_OPS,
            fops,
            empty_ram_bytes(),
        );
        init_inode_metadata(&i, 0, 0, 1, 0);
        *i.sb.lock() = Some(sb.clone());
        i
    }

    /// Create the UNIX98 pty multiplexor inode used by devtmpfs and devpts.
    ///
    /// Linux's `pty_init()` registers `ptmx_fops` on `(TTYAUX_MAJOR, 2)`, and
    /// `devpts::mknod_ptmx()` creates each devpts `ptmx` inode with that same
    /// device number.  Lupos binds character-device operations directly to the
    /// inode, so both halves must be installed here together.
    fn make_ptmx(sb: &SuperBlockRef) -> InodeRef {
        use crate::init::noinitramfs::{mkdev, new_encode_dev};

        let inode = make_special(sb, InodeKind::Chardev, 0o666, &PTMX_FILE_OPS);
        inode
            .rdev
            .store(new_encode_dev(mkdev(5, 2)) as u64, Ordering::Release);
        inode
    }

    fn make_devtmpfs_block(sb: &SuperBlockRef, mode: u32) -> InodeRef {
        let inode = Inode::new(
            sb.alloc_ino(),
            InodeKind::Blockdev,
            mode,
            &TMPFS_FILE_INODE_OPS,
            &crate::block::block_device::BLOCK_DEVICE_FILE_OPS,
            InodePrivate::Opaque(DEVTMPFS_BLOCK_INODE_MARKER),
        );
        // Linux stores the registered device's dev_t in i_rdev.  The native
        // Lupos BlockDevice registry currently has no dev_t field, so leave
        // rdev at zero rather than fabricating a major/minor; I/O binds through
        // the live registry name until that device-model ABI is available.
        *inode.sb.lock() = Some(sb.clone());
        inode
    }

    fn dir_map(
        dir: &InodeRef,
    ) -> Result<&spin::Mutex<alloc::collections::BTreeMap<alloc::string::String, InodeRef>>, i32>
    {
        match &dir.private {
            InodePrivate::RamDir(m) => Ok(m),
            _ => Err(EINVAL),
        }
    }
    fn tmpfs_create(dir: &InodeRef, name: &str, mode: u32) -> Result<InodeRef, i32> {
        let sb = dir.sb.lock().clone().ok_or(EINVAL)?;
        let i = make_reg(&sb, Some(dir), mode);
        insert_tmpfs_inode(dir, name, i)
    }
    fn tmpfs_mkdir(dir: &InodeRef, name: &str, mode: u32) -> Result<InodeRef, i32> {
        let sb = dir.sb.lock().clone().ok_or(EINVAL)?;
        let i = make_dir(&sb, Some(dir), mode);
        insert_tmpfs_inode(dir, name, i)
    }
    fn tmpfs_symlink(dir: &InodeRef, name: &str, target: &str, mode: u32) -> Result<InodeRef, i32> {
        let sb = dir.sb.lock().clone().ok_or(EINVAL)?;
        let i = make_symlink(&sb, Some(dir), mode, target);
        insert_tmpfs_inode(dir, name, i)
    }

    fn insert_tmpfs_inode(dir: &InodeRef, name: &str, inode: InodeRef) -> Result<InodeRef, i32> {
        let mut entries = dir_map(dir)?.lock();
        if entries.contains_key(name) {
            return Err(EEXIST);
        }
        entries.insert(alloc::string::String::from(name), inode.clone());
        drop(entries);
        if inode.kind == InodeKind::Directory {
            dir.nlink.fetch_add(1, Ordering::AcqRel);
        }
        let prev = dir.size.load(Ordering::Acquire);
        dir.size
            .store(prev.saturating_add(BOGO_DIRENT_SIZE), Ordering::Release);
        touch_inode_now(dir);
        touch_inode_now(&inode);
        Ok(inode)
    }

    fn tmpfs_readlink(inode: &InodeRef, buf: &mut [u8]) -> Result<usize, i32> {
        let bytes = match &inode.private {
            InodePrivate::RamBytes(bytes) => bytes.lock(),
            _ => return Err(EINVAL),
        };
        let n = bytes.len().min(buf.len());
        buf[..n].copy_from_slice(&bytes[..n]);
        Ok(n)
    }

    fn insert_child(
        parent: &crate::fs::types::DentryRef,
        name: &str,
        inode: InodeRef,
    ) -> Result<crate::fs::types::DentryRef, i32> {
        let parent_inode = parent.inode().ok_or(EINVAL)?;
        // Hold the parent dcache write side across the RamDir insertion and
        // d_instantiate, matching Linux's locked negative-dentry creation
        // path.  In particular, `/init` may have cached a negative `/dev/vda`
        // while waiting for virtio-blk; device registration must instantiate
        // that same dentry instead of treating it as an existing device.
        let mut children = parent.children.write();
        let (child, newly_hashed) = if let Some(existing) = children.get(name).cloned() {
            if existing.inode().is_some() {
                return Err(EEXIST);
            }
            (existing, false)
        } else {
            let child = d_alloc(name);
            *child.parent.lock() = Some(parent.clone());
            children.insert(String::from(name), child.clone());
            (child, true)
        };
        if let Err(errno) = insert_tmpfs_inode(&parent_inode, name, inode.clone()) {
            if newly_hashed {
                children.remove(name);
            }
            return Err(errno);
        }
        child.instantiate(inode);
        Ok(child)
    }

    fn populate_devtmpfs(sb: &SuperBlockRef) -> Result<(), i32> {
        let root = sb.root().ok_or(EINVAL)?;
        let console = &crate::init::rootfs::CONSOLE_FILE_OPS;
        // `st_rdev` in Linux `new_encode_dev()` form. Userspace relies on real
        // device numbers: Xorg's `xf86HasTTYs()` only enables VT/console
        // management (VT switch + `KDSETMODE(KD_GRAPHICS)`) when
        // `major(stat("/dev/tty0").st_rdev) == TTY_MAJOR` (4).
        let dev = |major: u32, minor: u32| {
            use crate::init::noinitramfs::{mkdev, new_encode_dev};
            new_encode_dev(mkdev(major, minor)) as u64
        };
        for (name, mode, rdev) in [
            ("console", 0o600, dev(5, 1)),
            ("tty", 0o666, dev(5, 0)),
            ("ttyS0", 0o620, dev(4, 64)),
        ] {
            let node = make_special(sb, InodeKind::Chardev, mode, console);
            node.rdev.store(rdev, Ordering::Release);
            insert_child(&root, name, node)?;
        }
        for minor in 0..=crate::linux_driver_abi::tty::VT_MAX_CONSOLES {
            let name = alloc::format!("tty{minor}");
            let node = make_special(sb, InodeKind::Chardev, 0o620, console);
            node.rdev.store(dev(4, minor), Ordering::Release);
            insert_child(&root, &name, node)?;
        }
        let kmsg = make_special(
            sb,
            InodeKind::Chardev,
            0o666,
            &crate::init::rootfs::DEV_KMSG_FILE_OPS,
        );
        kmsg.rdev.store(dev(1, 11), Ordering::Release);
        insert_child(&root, "kmsg", kmsg)?;
        for (name, minor) in [("null", 3), ("zero", 5), ("random", 8), ("urandom", 9)] {
            let node = make_special(sb, InodeKind::Chardev, 0o666, &TMPFS_FILE_OPS);
            node.rdev.store(dev(1, minor), Ordering::Release);
            insert_child(&root, name, node)?;
        }
        let full = make_special(
            sb,
            InodeKind::Chardev,
            0o666,
            &crate::init::rootfs::DEV_FULL_FILE_OPS,
        );
        full.rdev.store(dev(1, 7), Ordering::Release);
        insert_child(&root, "full", full)?;
        insert_child(&root, "ptmx", make_ptmx(sb))?;
        let root_inode = root.inode().ok_or(EINVAL)?;
        let pts = insert_child(&root, "pts", make_dir(sb, Some(&root_inode), 0o755))?;
        insert_child(&pts, "ptmx", make_ptmx(sb))?;
        let input = insert_child(&root, "input", make_dir(sb, Some(&root_inode), 0o755))?;
        for (idx, name) in ["event0", "event1"].into_iter().enumerate() {
            let node = make_special(sb, InodeKind::Chardev, 0o660, &EVDEV_FILE_OPS);
            node.rdev.store(dev(13, 64 + idx as u32), Ordering::Release);
            insert_child(&input, name, node)?;
        }
        let fb0 = make_special(sb, InodeKind::Chardev, 0o660, &FBDEV_FILE_OPS);
        fb0.rdev.store(dev(29, 0), Ordering::Release);
        insert_child(&root, "fb0", fb0)?;
        Ok(())
    }

    fn create_devtmpfs_block_node(name: &str) -> Result<(), i32> {
        let root = DEVTMPFS_ROOT.lock().clone().ok_or(EINVAL)?;
        let sb = root
            .inode()
            .and_then(|inode| inode.sb.lock().clone())
            .ok_or(EINVAL)?;
        let node_path = name
            .strip_prefix("/dev/")
            .or_else(|| name.strip_prefix("dev/"))
            .unwrap_or(name)
            .trim_matches('/');
        if node_path.is_empty() {
            return Err(EINVAL);
        }

        let mut parent = root;
        let mut components = node_path
            .split('/')
            .filter(|component| !component.is_empty())
            .peekable();
        while let Some(component) = components.next() {
            if component == "." || component == ".." {
                return Err(EINVAL);
            }
            let is_leaf = components.peek().is_none();
            if let Some(existing) = d_lookup(&parent, component) {
                if is_leaf {
                    // devtmpfs requests are idempotent for an already-published
                    // block node. A negative dentry is the normal result of
                    // userspace polling before device_add(); instantiate it.
                    // Never replace a positive user-created or differently
                    // typed inode at the requested path.
                    if let Some(inode) = existing.inode() {
                        return if inode.kind == InodeKind::Blockdev
                            && core::ptr::eq(
                                inode.fops,
                                &crate::block::block_device::BLOCK_DEVICE_FILE_OPS,
                            ) {
                            Ok(())
                        } else {
                            Err(EEXIST)
                        };
                    }
                    insert_child(&parent, component, make_devtmpfs_block(&sb, 0o600))?;
                    return Ok(());
                }
                let Some(inode) = existing.inode() else {
                    let parent_inode = parent.inode().ok_or(EINVAL)?;
                    parent = insert_child(
                        &parent,
                        component,
                        make_dir(&sb, Some(&parent_inode), 0o755),
                    )?;
                    continue;
                };
                if inode.kind != InodeKind::Directory {
                    return Err(EEXIST);
                }
                parent = existing;
                continue;
            }

            if is_leaf {
                insert_child(&parent, component, make_devtmpfs_block(&sb, 0o600))?;
                return Ok(());
            }

            let parent_inode = parent.inode().ok_or(EINVAL)?;
            parent = insert_child(
                &parent,
                component,
                make_dir(&sb, Some(&parent_inode), 0o755),
            )?;
        }
        Err(EINVAL)
    }

    /// Publish a newly registered block device into the active devtmpfs.
    /// `device_add()` ignores devtmpfs node-creation errors in Linux, so this
    /// intentionally remains a best-effort notification.
    pub fn publish_devtmpfs_block_device(name: &str) {
        let _ = create_devtmpfs_block_node(name);
    }

    fn remove_devtmpfs_block_node(name: &str) -> Result<(), i32> {
        let root = DEVTMPFS_ROOT.lock().clone().ok_or(EINVAL)?;
        let node_path = name
            .strip_prefix("/dev/")
            .or_else(|| name.strip_prefix("dev/"))
            .unwrap_or(name)
            .trim_matches('/');
        let (parent_path, leaf) = node_path.rsplit_once('/').unwrap_or(("", node_path));
        if leaf.is_empty() || leaf == "." || leaf == ".." {
            return Err(EINVAL);
        }

        let mut parent = root;
        for component in parent_path
            .split('/')
            .filter(|component| !component.is_empty())
        {
            if component == "." || component == ".." {
                return Err(EINVAL);
            }
            let next = d_lookup(&parent, component).ok_or(EINVAL)?;
            if next
                .inode()
                .is_none_or(|inode| inode.kind != InodeKind::Directory)
            {
                return Err(EINVAL);
            }
            parent = next;
        }

        let dentry = d_lookup(&parent, leaf).ok_or(EINVAL)?;
        let inode = dentry.inode().ok_or(EINVAL)?;
        if inode.kind != InodeKind::Blockdev
            || !matches!(
                &inode.private,
                InodePrivate::Opaque(marker) if *marker == DEVTMPFS_BLOCK_INODE_MARKER
            )
        {
            // `devtmpfs_delete_node()` only removes inodes it created.
            return Err(EPERM);
        }
        let parent_inode = parent.inode().ok_or(EINVAL)?;
        simple_unlink(&parent_inode, leaf, &inode)?;
        d_drop(&parent, leaf);
        Ok(())
    }

    /// Remove the kernel-created node corresponding to an unregistered block
    /// device.  User-created block nodes are preserved, matching dev_mynode().
    pub fn unpublish_devtmpfs_block_device(name: &str) {
        let _ = remove_devtmpfs_block_node(name);
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct TmpfsRootOptions {
        mode: u32,
        uid: u32,
        gid: u32,
    }

    impl Default for TmpfsRootOptions {
        fn default() -> Self {
            Self {
                mode: 0o755,
                uid: 0,
                gid: 0,
            }
        }
    }

    fn parse_tmpfs_root_options(data: &str) -> Result<TmpfsRootOptions, i32> {
        let mut options = TmpfsRootOptions::default();
        for option in data.split(',').filter(|option| !option.is_empty()) {
            let Some((name, value)) = option.split_once('=') else {
                continue;
            };
            match name {
                "mode" => {
                    if value.is_empty() {
                        return Err(EINVAL);
                    }
                    options.mode = u32::from_str_radix(value, 8).map_err(|_| EINVAL)?;
                    if options.mode & !0o7777 != 0 {
                        return Err(EINVAL);
                    }
                }
                "uid" => options.uid = value.parse::<u32>().map_err(|_| EINVAL)?,
                "gid" => options.gid = value.parse::<u32>().map_err(|_| EINVAL)?,
                _ => {}
            }
        }
        Ok(options)
    }

    fn mount_named_with_root_options(
        fs_name: &'static str,
        options: TmpfsRootOptions,
    ) -> Result<SuperBlockRef, i32> {
        let sb = SuperBlock::alloc(fs_name, TMPFS_MAGIC, &TMPFS_SUPER_OPS);
        let root_inode = make_dir(&sb, None, options.mode);
        root_inode.uid.store(options.uid, Ordering::Release);
        root_inode.gid.store(options.gid, Ordering::Release);
        let root = d_alloc("/");
        root.instantiate(root_inode);
        *sb.root.lock() = Some(root);
        Ok(sb)
    }

    fn mount_named(fs_name: &'static str) -> Result<SuperBlockRef, i32> {
        mount_named_with_root_options(fs_name, TmpfsRootOptions::default())
    }

    pub fn mount(_source: &str, _flags: u64, data: &str) -> Result<SuperBlockRef, i32> {
        mount_named_with_root_options("tmpfs", parse_tmpfs_root_options(data)?)
    }

    pub fn mount_devtmpfs(_source: &str, _flags: u64, _data: &str) -> Result<SuperBlockRef, i32> {
        // `devtmpfs_get_tree()` returns the one internal devtmpfs superblock
        // for every mount.  Keep the initialization lock through the registry
        // snapshot so a concurrent duplicate mount cannot observe a root that
        // missed devices registered before publication became active.
        let mut mounted = DEVTMPFS_SUPERBLOCK.lock();
        if let Some(sb) = mounted.as_ref() {
            return Ok(sb.clone());
        }
        let sb = mount_named("devtmpfs")?;
        populate_devtmpfs(&sb)?;
        *DEVTMPFS_ROOT.lock() = sb.root();
        // Close the registration-before-mount window.  Devices registered
        // after this snapshot publish themselves from register_block_device().
        crate::block::block_device::publish_registered_block_devices_to_devtmpfs();
        *mounted = Some(sb.clone());
        Ok(sb)
    }

    pub fn mount_devpts(_source: &str, _flags: u64, _data: &str) -> Result<SuperBlockRef, i32> {
        let sb = mount_named("devpts")?;
        let root = sb.root().ok_or(EINVAL)?;
        insert_child(&root, "ptmx", make_ptmx(&sb))?;
        Ok(sb)
    }

    pub fn register() {
        let _ = register_filesystem(FileSystemType {
            name: "tmpfs",
            mount,
            fs_flags: 0,
        });
        let _ = register_filesystem(FileSystemType {
            name: "devtmpfs",
            mount: mount_devtmpfs,
            fs_flags: 0,
        });
        let _ = register_filesystem(FileSystemType {
            name: "devpts",
            mount: mount_devpts,
            fs_flags: 0,
        });
    }

    #[cfg(test)]
    mod tests {
        use alloc::boxed::Box;
        use alloc::sync::Arc;

        use super::*;
        use crate::fs::dcache::d_lookup;
        use crate::include::uapi::stat::S_ISGID;
        use crate::kernel::capability::KernelCapT;
        use crate::kernel::cred::{Cred, GroupInfo, INIT_CRED, KGid, KUid};
        use crate::kernel::{sched, task::TaskStruct};

        fn child_kind(sb: &SuperBlockRef, name: &str) -> InodeKind {
            let root = sb.root().expect("root");
            d_lookup(&root, name)
                .and_then(|d| d.inode())
                .map(|inode| inode.kind)
                .expect("devtmpfs child")
        }

        fn install_current(current: &mut TaskStruct, cred: &Cred) -> *mut TaskStruct {
            let previous = unsafe { sched::get_current() };
            current.pid = 4343;
            current.tgid = 4343;
            current.cred = cred as *const Cred;
            current.m27.real_cred = cred as *const Cred;
            unsafe { sched::set_current(current as *mut TaskStruct) };
            previous
        }

        fn test_cred(uid: u32, gid: u32) -> Cred {
            Cred {
                usage: core::sync::atomic::AtomicUsize::new(1),
                uid: KUid(uid),
                gid: KGid(gid),
                suid: KUid(uid),
                sgid: KGid(gid),
                euid: KUid(uid),
                egid: KGid(gid),
                fsuid: KUid(uid),
                fsgid: KGid(gid),
                cap_inheritable: KernelCapT::empty(),
                cap_permitted: KernelCapT::empty(),
                cap_effective: KernelCapT::empty(),
                cap_bset: KernelCapT::empty(),
                cap_ambient: KernelCapT::empty(),
                securebits: 0,
                group_info: GroupInfo::default(),
                user_ns: core::ptr::null(),
            }
        }

        #[test]
        fn mounted_devtmpfs_contains_core_device_nodes() {
            let sb = mount_devtmpfs("devtmpfs", 0, "").expect("mount devtmpfs");
            for name in [
                "console", "tty", "tty1", "ttyS0", "kmsg", "null", "zero", "full", "random",
                "urandom", "ptmx",
            ] {
                assert_eq!(child_kind(&sb, name), InodeKind::Chardev, "{name}");
            }
            let root = sb.root().expect("root");
            let kmsg = d_lookup(&root, "kmsg").expect("kmsg dentry");
            let kmsg_inode = kmsg.inode().expect("kmsg inode");
            assert_eq!(kmsg_inode.fops.name, "dev_kmsg");
            assert!(
                kmsg_inode.fops.poll.is_some(),
                "/dev/kmsg must be pollable for systemd-journald"
            );
            let full = d_lookup(&root, "full")
                .and_then(|dentry| dentry.inode())
                .expect("full inode");
            assert_eq!(full.fops.name, "dev_full");
            assert_eq!(
                full.rdev.load(Ordering::Acquire),
                crate::init::noinitramfs::new_encode_dev(crate::init::noinitramfs::mkdev(1, 7))
                    as u64
            );
            let input = d_lookup(&root, "input").expect("input dentry");
            assert_eq!(
                input.inode().expect("input inode").kind,
                InodeKind::Directory
            );
            let event0 = d_lookup(&input, "event0").expect("event0 dentry");
            assert_eq!(
                event0.inode().expect("event0 inode").kind,
                InodeKind::Chardev
            );

            let pts = d_lookup(&root, "pts").expect("pts dentry");
            assert_eq!(pts.inode().expect("pts inode").kind, InodeKind::Directory);
            let ptmx = d_lookup(&pts, "ptmx").expect("pts ptmx dentry");
            assert_eq!(
                ptmx.inode().expect("pts ptmx inode").kind,
                InodeKind::Chardev
            );
        }

        #[test]
        fn mounted_devpts_contains_ptmx() {
            let sb = mount_devpts("devpts", 0, "").expect("mount devpts");
            assert_eq!(child_kind(&sb, "ptmx"), InodeKind::Chardev);
        }

        #[test]
        fn tmpfs_mount_applies_root_mode_uid_and_gid_options() {
            let sb = mount("tmpfs", 0, "size=10M,mode=0700,uid=1000,gid=1001")
                .expect("tmpfs mount options");
            let root = sb.root().expect("tmpfs root");
            let inode = root.inode().expect("tmpfs root inode");

            assert_eq!(inode.mode.load(Ordering::Acquire) & 0o7777, 0o700);
            assert_eq!(inode.uid.load(Ordering::Acquire), 1000);
            assert_eq!(inode.gid.load(Ordering::Acquire), 1001);
        }

        #[test]
        fn tmpfs_mount_rejects_malformed_root_identity_options() {
            assert!(matches!(mount("tmpfs", 0, "mode=not-octal"), Err(EINVAL)));
            assert!(matches!(mount("tmpfs", 0, "uid=not-a-number"), Err(EINVAL)));
            assert!(matches!(mount("tmpfs", 0, "gid="), Err(EINVAL)));
        }

        #[test]
        fn tmpfs_create_and_mkdir_reject_duplicates_without_replacing_inodes() {
            let sb = mount("tmpfs", 0, "").expect("mount tmpfs");
            let root = sb.root().expect("root");
            let root_inode = root.inode().expect("root inode");

            let created = tmpfs_create(&root_inode, "state", 0o644).expect("create");
            assert!(matches!(
                tmpfs_create(&root_inode, "state", 0o600),
                Err(EEXIST)
            ));
            let looked_up = root_inode.ops.lookup.unwrap()(&root_inode, "state").expect("lookup");
            assert!(Arc::ptr_eq(&created, &looked_up));
            assert_eq!(looked_up.kind, InodeKind::Regular);

            let dir = tmpfs_mkdir(&root_inode, "units", 0o755).expect("mkdir");
            assert!(matches!(
                tmpfs_mkdir(&root_inode, "units", 0o700),
                Err(EEXIST)
            ));
            let looked_up = root_inode.ops.lookup.unwrap()(&root_inode, "units").expect("lookup");
            assert!(Arc::ptr_eq(&dir, &looked_up));
            assert_eq!(looked_up.kind, InodeKind::Directory);
        }

        #[test]
        fn tmpfs_symlink_round_trips_systemd_invocation_link() {
            let sb = mount("tmpfs", 0, "").expect("mount tmpfs");
            let root = sb.root().expect("root");
            let root_inode = root.inode().expect("root inode");
            let units = tmpfs_mkdir(&root_inode, "units", 0o755).expect("units dir");
            let link = tmpfs_symlink(
                &units,
                ".#invocation:systemd-vconsole-setup.servicee13514345844c9fe",
                "ad048641fc1f44bca483d91fc0b0323e",
                0o777,
            )
            .expect("tmpfs symlink");

            assert_eq!(link.kind, InodeKind::Symlink);
            let mut buf = [0u8; 64];
            let n = link.ops.readlink.unwrap()(&link, &mut buf).expect("readlink");
            assert_eq!(&buf[..n], b"ad048641fc1f44bca483d91fc0b0323e");
            assert!(matches!(
                tmpfs_symlink(
                    &units,
                    ".#invocation:systemd-vconsole-setup.servicee13514345844c9fe",
                    "duplicate",
                    0o777,
                ),
                Err(EEXIST)
            ));
        }

        #[test]
        fn tmpfs_create_uses_current_fsuid_and_fsgid() {
            let sb = mount("tmpfs", 0, "").expect("mount tmpfs");
            let root = sb.root().expect("root");
            let root_inode = root.inode().expect("root inode");
            let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
            let cred = Box::new(test_cred(977, 977));
            let previous = install_current(&mut current, &cred);

            let inode = tmpfs_create(&root_inode, "networkd", 0o640).expect("create");

            unsafe { sched::set_current(previous) };
            current.cred = &raw const INIT_CRED;
            assert_eq!(inode.uid.load(Ordering::Acquire), 977);
            assert_eq!(inode.gid.load(Ordering::Acquire), 977);
            assert_eq!(inode.mode.load(Ordering::Acquire) & 0o7777, 0o640);
        }

        #[test]
        fn tmpfs_directory_create_inherits_parent_setgid_gid_and_mode() {
            let sb = mount("tmpfs", 0, "").expect("mount tmpfs");
            let root = sb.root().expect("root");
            let root_inode = root.inode().expect("root inode");
            root_inode.gid.store(77, Ordering::Release);
            root_inode.mode.store(
                root_inode.mode.load(Ordering::Acquire) | S_ISGID,
                Ordering::Release,
            );

            let inode = tmpfs_mkdir(&root_inode, "links", 0o750).expect("mkdir");

            assert_eq!(inode.gid.load(Ordering::Acquire), 77);
            assert_eq!(inode.mode.load(Ordering::Acquire) & 0o7777, S_ISGID | 0o750);
            assert_eq!(root_inode.nlink.load(Ordering::Acquire), 3);
            assert_eq!(
                root_inode.size.load(Ordering::Acquire),
                3 * BOGO_DIRENT_SIZE
            );
            assert_eq!(inode.size.load(Ordering::Acquire), 2 * BOGO_DIRENT_SIZE);
        }
    }
}

pub use tmpfs_vfs::{
    TMPFS_DIR_FILE_OPS, TMPFS_DIR_INODE_OPS, TMPFS_FILE_INODE_OPS, TMPFS_FILE_OPS, TMPFS_SUPER_OPS,
    mount, mount_devpts, mount_devtmpfs, publish_devtmpfs_block_device, register,
    unpublish_devtmpfs_block_device,
};
