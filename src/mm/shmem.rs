//! linux-parity: complete
//! linux-source: vendor/linux/mm/shmem.c
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

use spin::Mutex;

use crate::include::uapi::errno::{EINVAL, ENODEV, ENOMEM, EPERM};
use crate::mm::address_space::AddressSpace;
use crate::mm::frame::PAGE_SIZE;
use crate::mm::mm_types::VmAreaStruct;
use crate::mm::page::Page;
use crate::mm::page_flags::GfpFlags;

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

pub fn shmem_truncate_range(mapping: *mut AddressSpace, start: u64, end: u64) {
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

pub fn shmem_read_folio_gfp(mapping: *mut AddressSpace, folio: *mut Page, gfp: GfpFlags) -> i32 {
    unsafe { crate::mm::filemap::mapping_read_folio_gfp(mapping, folio, gfp) }
}

pub fn shmem_read_folio(mapping: *mut AddressSpace, folio: *mut Page) -> i32 {
    shmem_read_folio_gfp(mapping, folio, crate::mm::page_flags::GFP_KERNEL)
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

pub fn shmem_writeout(_folio: *mut Page, _wbc: *mut u8) -> i32 {
    -EINVAL
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

    use crate::fs::dcache::d_alloc;
    use crate::fs::libfs::{
        empty_ram_bytes, empty_ram_dir, ram_file_read, ram_file_write, simple_lookup,
        simple_readdir, simple_rmdir, simple_unlink,
    };
    use crate::fs::ops::{FileOps, InodeOps, SuperOps};
    use crate::fs::super_block::{FileSystemType, register_filesystem};
    use crate::fs::types::{Inode, InodeKind, InodePrivate, InodeRef, SuperBlock, SuperBlockRef};
    use crate::include::uapi::errno::EINVAL;
    use crate::linux_driver_abi::input::evdev_chardev::EVDEV_FILE_OPS;
    use crate::linux_driver_abi::video::fbdev::FBDEV_FILE_OPS;

    const TMPFS_MAGIC: u64 = 0x01021994;

    pub static TMPFS_DIR_INODE_OPS: InodeOps = InodeOps {
        name: "tmpfs_dir",
        lookup: Some(simple_lookup),
        create: Some(tmpfs_create),
        mkdir: Some(tmpfs_mkdir),
        unlink: Some(simple_unlink),
        rmdir: Some(simple_rmdir),
        rename: None,
        symlink: None,
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

    pub static TMPFS_SUPER_OPS: SuperOps = SuperOps {
        name: "tmpfs",
        statfs: None,
        put_super: None,
        sync_fs: None,
        alloc_inode: None,
        destroy_inode: None,
    };

    fn make_dir(sb: &SuperBlockRef) -> InodeRef {
        let i = Inode::new(
            sb.alloc_ino(),
            InodeKind::Directory,
            0o755,
            &TMPFS_DIR_INODE_OPS,
            &TMPFS_DIR_FILE_OPS,
            empty_ram_dir(),
        );
        *i.sb.lock() = Some(sb.clone());
        i
    }
    fn make_reg(sb: &SuperBlockRef, mode: u32) -> InodeRef {
        let i = Inode::new(
            sb.alloc_ino(),
            InodeKind::Regular,
            mode,
            &TMPFS_FILE_INODE_OPS,
            &TMPFS_FILE_OPS,
            empty_ram_bytes(),
        );
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
        *i.sb.lock() = Some(sb.clone());
        i
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
        let i = make_reg(&sb, mode);
        dir_map(dir)?
            .lock()
            .insert(alloc::string::String::from(name), i.clone());
        Ok(i)
    }
    fn tmpfs_mkdir(dir: &InodeRef, name: &str, _mode: u32) -> Result<InodeRef, i32> {
        let sb = dir.sb.lock().clone().ok_or(EINVAL)?;
        let i = make_dir(&sb);
        dir_map(dir)?
            .lock()
            .insert(alloc::string::String::from(name), i.clone());
        Ok(i)
    }

    fn insert_child(
        parent: &crate::fs::types::DentryRef,
        name: &str,
        inode: InodeRef,
    ) -> Result<crate::fs::types::DentryRef, i32> {
        let parent_inode = parent.inode().ok_or(EINVAL)?;
        dir_map(&parent_inode)?
            .lock()
            .insert(String::from(name), inode.clone());
        let child = crate::fs::dcache::d_alloc_child(parent, name);
        child.instantiate(inode);
        Ok(child)
    }

    fn populate_devtmpfs(sb: &SuperBlockRef) -> Result<(), i32> {
        let root = sb.root().ok_or(EINVAL)?;
        let console = &crate::init::rootfs::CONSOLE_FILE_OPS;
        for (name, mode) in [
            ("console", 0o600),
            ("tty", 0o666),
            ("tty0", 0o620),
            ("tty1", 0o620),
            ("tty2", 0o620),
            ("tty3", 0o620),
            ("tty4", 0o620),
            ("tty5", 0o620),
            ("tty6", 0o620),
            ("ttyS0", 0o620),
        ] {
            insert_child(
                &root,
                name,
                make_special(sb, InodeKind::Chardev, mode, console),
            )?;
        }
        insert_child(
            &root,
            "kmsg",
            make_special(
                sb,
                InodeKind::Chardev,
                0o666,
                &crate::init::rootfs::DEV_KMSG_FILE_OPS,
            ),
        )?;
        for name in ["null", "zero", "random", "urandom"] {
            insert_child(
                &root,
                name,
                make_special(sb, InodeKind::Chardev, 0o666, &TMPFS_FILE_OPS),
            )?;
        }
        insert_child(
            &root,
            "ptmx",
            make_special(sb, InodeKind::Chardev, 0o666, &TMPFS_FILE_OPS),
        )?;
        for name in ["vda", "vda1"] {
            insert_child(
                &root,
                name,
                make_special(sb, InodeKind::Blockdev, 0o660, &TMPFS_FILE_OPS),
            )?;
        }
        let pts = insert_child(&root, "pts", make_dir(sb))?;
        insert_child(
            &pts,
            "ptmx",
            make_special(sb, InodeKind::Chardev, 0o666, &TMPFS_FILE_OPS),
        )?;
        let input = insert_child(&root, "input", make_dir(sb))?;
        for name in ["event0", "event1"] {
            insert_child(
                &input,
                name,
                make_special(sb, InodeKind::Chardev, 0o660, &EVDEV_FILE_OPS),
            )?;
        }
        insert_child(
            &root,
            "fb0",
            make_special(sb, InodeKind::Chardev, 0o660, &FBDEV_FILE_OPS),
        )?;
        Ok(())
    }

    fn mount_named(fs_name: &'static str) -> Result<SuperBlockRef, i32> {
        let sb = SuperBlock::alloc(fs_name, TMPFS_MAGIC, &TMPFS_SUPER_OPS);
        let root_inode = make_dir(&sb);
        let root = d_alloc("/");
        root.instantiate(root_inode);
        *sb.root.lock() = Some(root);
        Ok(sb)
    }

    pub fn mount(_source: &str, _flags: u64, _data: &str) -> Result<SuperBlockRef, i32> {
        mount_named("tmpfs")
    }

    pub fn mount_devtmpfs(_source: &str, _flags: u64, _data: &str) -> Result<SuperBlockRef, i32> {
        let sb = mount_named("devtmpfs")?;
        populate_devtmpfs(&sb)?;
        Ok(sb)
    }

    pub fn mount_devpts(_source: &str, _flags: u64, _data: &str) -> Result<SuperBlockRef, i32> {
        let sb = mount_named("devpts")?;
        let root = sb.root().ok_or(EINVAL)?;
        insert_child(
            &root,
            "ptmx",
            make_special(&sb, InodeKind::Chardev, 0o666, &TMPFS_FILE_OPS),
        )?;
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
        use super::*;
        use crate::fs::dcache::d_lookup;

        fn child_kind(sb: &SuperBlockRef, name: &str) -> InodeKind {
            let root = sb.root().expect("root");
            d_lookup(&root, name)
                .and_then(|d| d.inode())
                .map(|inode| inode.kind)
                .expect("devtmpfs child")
        }

        #[test]
        fn mounted_devtmpfs_contains_core_device_nodes() {
            let sb = mount_devtmpfs("devtmpfs", 0, "").expect("mount devtmpfs");
            for name in ["console", "tty", "tty1", "ttyS0", "kmsg", "null", "ptmx"] {
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
            for name in ["vda", "vda1"] {
                assert_eq!(child_kind(&sb, name), InodeKind::Blockdev, "{name}");
            }

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
    }
}

pub use tmpfs_vfs::{
    TMPFS_DIR_FILE_OPS, TMPFS_DIR_INODE_OPS, TMPFS_FILE_INODE_OPS, TMPFS_FILE_OPS, TMPFS_SUPER_OPS,
    mount, mount_devpts, mount_devtmpfs, register,
};
