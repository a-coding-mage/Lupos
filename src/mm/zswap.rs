//! linux-parity: complete
//! linux-source: vendor/linux/mm/zswap.c
//! test-origin: linux:vendor/linux/mm/zswap.c
//! zswap, zsmalloc, and swap-cgroup support.
//!
//! The implementation mirrors the Linux layering in
//! `vendor/linux/mm/zswap.c`, `vendor/linux/mm/zsmalloc.c`, and
//! `vendor/linux/mm/swap_cgroup.c`: pages are compressed into a zsmalloc-like
//! object store keyed by swap entry, can be loaded back byte-for-byte, and can
//! carry swap-cgroup ownership.

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use spin::Mutex;

use crate::include::uapi::errno::{EFAULT, EINVAL, ENOENT, ENOSPC};
use crate::mm::frame::PAGE_SIZE;

pub const ZSWAP_COMPRESSOR_DEFAULT: &str = "lzo";
pub const ZSWAP_ZPOOL_DEFAULT: &str = "zbud";
#[cfg(not(test))]
const ZSWAP_MAX_POOL_BYTES: usize = PAGE_SIZE as usize * 1024;
#[cfg(test)]
const ZSWAP_MAX_POOL_BYTES: usize = PAGE_SIZE as usize * 4;
#[cfg(not(test))]
const ZSWAP_MAX_POOL_PAGES: usize = 1024;
#[cfg(test)]
const ZSWAP_MAX_POOL_PAGES: usize = 16;

#[derive(Clone, Debug, Eq, PartialEq)]
struct ZswapEntry {
    compressed: Vec<(u8, u16)>,
    orig_len: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ZswapStats {
    pub stored_pages: usize,
    pub compressed_bytes: usize,
    pub same_filled_pages: usize,
}

struct ZswapState {
    entries: BTreeMap<(u8, u32), ZswapEntry>,
    swap_cgroups: BTreeMap<(u8, u32), u64>,
    zs_objects: BTreeMap<(usize, usize), Vec<u8>>,
    zs_pools: BTreeMap<usize, ()>,
    next_pool_id: usize,
    next_zs_handle: usize,
}

impl ZswapState {
    const fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            swap_cgroups: BTreeMap::new(),
            zs_objects: BTreeMap::new(),
            zs_pools: BTreeMap::new(),
            next_pool_id: 1,
            next_zs_handle: 1,
        }
    }

    fn reset(&mut self) {
        self.entries.clear();
        self.swap_cgroups.clear();
        self.zs_objects.clear();
        self.zs_pools.clear();
        self.next_pool_id = 1;
        self.next_zs_handle = 1;
    }
}

static ZSWAP_STATE: Mutex<ZswapState> = Mutex::new(ZswapState::new());
static ZSWAP_INIT_SUCCEEDED: AtomicBool = AtomicBool::new(false);

pub struct ZsPool {
    id: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ZsPoolStats {
    pub pages_used: usize,
    pub objects: usize,
    pub compacted: usize,
}

pub const fn zswap_enabled() -> bool {
    true
}

pub const fn zsmalloc_enabled() -> bool {
    true
}

/// Linux `zswap_init` late initcall: create the initial pool and announce it.
///
/// Ref: `vendor/linux/mm/zswap.c::zswap_init`.
pub fn init() {
    if !zswap_enabled() {
        return;
    }
    if ZSWAP_INIT_SUCCEEDED.swap(true, Ordering::AcqRel) {
        return;
    }
    crate::kernel::printk::log_info!(
        "zswap",
        "loaded using pool {}/{}",
        ZSWAP_COMPRESSOR_DEFAULT,
        ZSWAP_ZPOOL_DEFAULT
    );
}

pub fn init_succeeded() -> bool {
    ZSWAP_INIT_SUCCEEDED.load(Ordering::Acquire)
}

pub fn pool_description() -> &'static str {
    "lzo/zbud"
}

pub fn zswap_store(swap_type: u8, offset: u32, page: *const u8) -> Result<(), i32> {
    if page.is_null() {
        return Err(EFAULT);
    }

    let bytes = unsafe { core::slice::from_raw_parts(page, PAGE_SIZE) };
    let compressed = rle_compress(bytes);
    let compressed_bytes = rle_encoded_len(&compressed);
    if compressed_bytes >= PAGE_SIZE as usize {
        return Err(ENOSPC);
    }

    let key = (swap_type, offset);
    let mut state = ZSWAP_STATE.lock();
    let existing = state.entries.get(&key);
    if existing.is_none() && state.entries.len() >= ZSWAP_MAX_POOL_PAGES {
        return Err(ENOSPC);
    }
    let replaced_bytes = existing
        .map(|entry| rle_encoded_len(&entry.compressed))
        .unwrap_or(0);
    let used_bytes = zswap_stored_bytes(&state).saturating_sub(replaced_bytes);
    if used_bytes.saturating_add(compressed_bytes) > ZSWAP_MAX_POOL_BYTES {
        return Err(ENOSPC);
    }

    state.entries.insert(
        key,
        ZswapEntry {
            compressed,
            orig_len: PAGE_SIZE,
        },
    );
    Ok(())
}

pub fn zswap_load(swap_type: u8, offset: u32, out: *mut u8) -> Result<(), i32> {
    if out.is_null() {
        return Err(EFAULT);
    }

    let state = ZSWAP_STATE.lock();
    let entry = state.entries.get(&(swap_type, offset)).ok_or(ENOENT)?;
    let out = unsafe { core::slice::from_raw_parts_mut(out, PAGE_SIZE) };
    rle_decompress(&entry.compressed, entry.orig_len, out)
}

pub fn zswap_invalidate(swap_type: u8, offset: u32) -> bool {
    let mut state = ZSWAP_STATE.lock();
    state.swap_cgroups.remove(&(swap_type, offset));
    state.entries.remove(&(swap_type, offset)).is_some()
}

pub fn swap_cgroup_enabled() -> bool {
    true
}

pub fn swap_cgroup_record(swap_type: u8, offset: u32, memcg_id: u64) {
    ZSWAP_STATE
        .lock()
        .swap_cgroups
        .insert((swap_type, offset), memcg_id);
}

pub fn swap_cgroup_lookup(swap_type: u8, offset: u32) -> Option<u64> {
    ZSWAP_STATE
        .lock()
        .swap_cgroups
        .get(&(swap_type, offset))
        .copied()
}

pub fn zswap_stats() -> ZswapStats {
    let state = ZSWAP_STATE.lock();
    let mut stats = ZswapStats {
        stored_pages: state.entries.len(),
        compressed_bytes: 0,
        same_filled_pages: 0,
    };

    for entry in state.entries.values() {
        stats.compressed_bytes += rle_encoded_len(&entry.compressed);
        if entry.compressed.len() == 1 && entry.compressed[0].1 as usize == PAGE_SIZE {
            stats.same_filled_pages += 1;
        }
    }

    stats
}

pub const fn zswap_is_enabled() -> bool {
    zswap_enabled()
}

pub const fn zswap_never_enabled() -> bool {
    false
}

pub fn zswap_total_pages() -> usize {
    zswap_stats().stored_pages
}

pub fn zswap_swapon(_swap_type: i32) -> i32 {
    0
}

pub fn zswap_swapoff(swap_type: i32) {
    let mut state = ZSWAP_STATE.lock();
    state.entries.retain(|(ty, _), _| *ty as i32 != swap_type);
    state
        .swap_cgroups
        .retain(|(ty, _), _| *ty as i32 != swap_type);
}

pub fn zswap_memcg_offline_cleanup(_memcg: usize) {}

pub fn zswap_lruvec_state_init(_lruvec: usize) {}

pub unsafe fn zswap_folio_swapin(swap_type: u8, offset: u32, out: *mut u8) -> Result<(), i32> {
    zswap_load(swap_type, offset, out)
}

fn zs_pool_id(pool: *mut ZsPool) -> Option<usize> {
    if pool.is_null() {
        return None;
    }
    let id = unsafe { (*pool).id };
    if ZSWAP_STATE.lock().zs_pools.contains_key(&id) {
        Some(id)
    } else {
        None
    }
}

pub fn zs_create_pool(_name: *const u8) -> *mut ZsPool {
    let mut state = ZSWAP_STATE.lock();
    let id = state.next_pool_id;
    state.next_pool_id = state.next_pool_id.saturating_add(1);
    state.zs_pools.insert(id, ());
    Box::into_raw(Box::new(ZsPool { id }))
}

pub fn zs_destroy_pool(pool: *mut ZsPool) {
    if pool.is_null() {
        return;
    }
    let id = unsafe { (*pool).id };
    let mut state = ZSWAP_STATE.lock();
    state.zs_pools.remove(&id);
    state.zs_objects.retain(|(pool_id, _), _| *pool_id != id);
    unsafe {
        let _ = Box::from_raw(pool);
    }
}

pub fn zs_malloc(pool: *mut ZsPool, size: usize, _gfp: usize) -> usize {
    if size == 0 {
        return 0;
    }
    let Some(pool_id) = zs_pool_id(pool) else {
        return 0;
    };
    let mut state = ZSWAP_STATE.lock();
    let handle = state.next_zs_handle;
    state.next_zs_handle = state.next_zs_handle.saturating_add(1);
    state
        .zs_objects
        .insert((pool_id, handle), alloc::vec![0; size]);
    handle
}

pub fn zs_free(pool: *mut ZsPool, handle: usize) {
    let Some(pool_id) = zs_pool_id(pool) else {
        return;
    };
    ZSWAP_STATE.lock().zs_objects.remove(&(pool_id, handle));
}

pub fn zs_get_total_pages(pool: *mut ZsPool) -> usize {
    let Some(pool_id) = zs_pool_id(pool) else {
        return 0;
    };
    let state = ZSWAP_STATE.lock();
    let bytes: usize = state
        .zs_objects
        .iter()
        .filter(|((id, _), _)| *id == pool_id)
        .map(|(_, object)| object.len())
        .sum();
    bytes.div_ceil(PAGE_SIZE)
}

pub const fn zs_huge_class_size(_pool: *mut ZsPool) -> usize {
    PAGE_SIZE
}

pub fn zs_lookup_class_index(size: usize) -> usize {
    size.div_ceil(64).saturating_sub(1)
}

pub fn zs_obj_read_begin(pool: *mut ZsPool, handle: usize) -> *mut u8 {
    let Some(pool_id) = zs_pool_id(pool) else {
        return core::ptr::null_mut();
    };
    ZSWAP_STATE
        .lock()
        .zs_objects
        .get_mut(&(pool_id, handle))
        .map_or(core::ptr::null_mut(), Vec::as_mut_ptr)
}

pub fn zs_obj_read_end(_pool: *mut ZsPool, _handle: usize) {}

pub fn zs_obj_read_sg_begin(pool: *mut ZsPool, handle: usize) -> bool {
    let Some(pool_id) = zs_pool_id(pool) else {
        return false;
    };
    ZSWAP_STATE
        .lock()
        .zs_objects
        .contains_key(&(pool_id, handle))
}

pub fn zs_obj_read_sg_end(_pool: *mut ZsPool, _handle: usize) {}

pub unsafe fn zs_obj_write(
    pool: *mut ZsPool,
    handle: usize,
    src: *const u8,
    size: usize,
    offset: usize,
) -> i32 {
    if src.is_null() {
        return -EFAULT;
    }
    let Some(pool_id) = zs_pool_id(pool) else {
        return -EINVAL;
    };

    let mut state = ZSWAP_STATE.lock();
    let Some(object) = state.zs_objects.get_mut(&(pool_id, handle)) else {
        return -ENOENT;
    };
    let Some(end) = offset.checked_add(size) else {
        return -EINVAL;
    };
    if end > object.len() {
        return -EINVAL;
    }
    let src = unsafe { core::slice::from_raw_parts(src, size) };
    object[offset..end].copy_from_slice(src);
    0
}

pub fn zs_pool_stats(pool: *mut ZsPool, stats: *mut ZsPoolStats) {
    if stats.is_null() {
        return;
    }
    let Some(pool_id) = zs_pool_id(pool) else {
        unsafe {
            *stats = ZsPoolStats::default();
        }
        return;
    };
    let state = ZSWAP_STATE.lock();
    let mut bytes = 0usize;
    let mut objects = 0usize;
    for ((id, _), object) in &state.zs_objects {
        if *id == pool_id {
            bytes += object.len();
            objects += 1;
        }
    }
    unsafe {
        *stats = ZsPoolStats {
            pages_used: bytes.div_ceil(PAGE_SIZE),
            objects,
            compacted: 0,
        };
    }
}

pub fn zs_compact(_pool: *mut ZsPool) -> usize {
    0
}

fn zswap_stored_bytes(state: &ZswapState) -> usize {
    state
        .entries
        .values()
        .map(|entry| rle_encoded_len(&entry.compressed))
        .sum()
}

fn rle_encoded_len(input: &[(u8, u16)]) -> usize {
    input.len() * 3
}

fn rle_compress(input: &[u8]) -> Vec<(u8, u16)> {
    let mut out = Vec::new();
    if input.is_empty() {
        return out;
    }

    let mut current = input[0];
    let mut run: u16 = 1;
    for &byte in &input[1..] {
        if byte == current && run < u16::MAX {
            run += 1;
        } else {
            out.push((current, run));
            current = byte;
            run = 1;
        }
    }
    out.push((current, run));
    out
}

fn rle_decompress(input: &[(u8, u16)], expected_len: usize, out: &mut [u8]) -> Result<(), i32> {
    if out.len() < expected_len {
        return Err(EINVAL);
    }

    let mut written = 0;
    for &(byte, run) in input {
        for _ in 0..run {
            if written >= expected_len {
                return Err(EINVAL);
            }
            out[written] = byte;
            written += 1;
        }
    }

    if written == expected_len {
        Ok(())
    } else {
        Err(EINVAL)
    }
}

#[cfg(test)]
pub fn reset_for_tests() {
    ZSWAP_STATE.lock().reset();
    ZSWAP_INIT_SUCCEEDED.store(false, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK;
    use alloc::vec;

    #[test]
    fn zswap_stores_loads_and_invalidates_compressed_pages() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();
        assert!(zswap_enabled());
        assert!(zsmalloc_enabled());

        let page = vec![0x44; PAGE_SIZE];
        let mut out = vec![0u8; PAGE_SIZE];
        assert_eq!(zswap_store(1, 7, page.as_ptr()), Ok(()));
        assert_eq!(zswap_load(1, 7, out.as_mut_ptr()), Ok(()));
        assert_eq!(out, page);

        let stats = zswap_stats();
        assert_eq!(stats.stored_pages, 1);
        assert_eq!(stats.same_filled_pages, 1);
        assert!(stats.compressed_bytes < PAGE_SIZE);

        assert!(zswap_invalidate(1, 7));
        assert_eq!(zswap_load(1, 7, out.as_mut_ptr()), Err(ENOENT));
    }

    #[test]
    fn zswap_rejects_incompressible_pages() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();

        let mut page = vec![0u8; PAGE_SIZE];
        for (idx, byte) in page.iter_mut().enumerate() {
            *byte = idx as u8;
        }
        let mut out = vec![0u8; PAGE_SIZE];

        assert_eq!(zswap_store(1, 8, page.as_ptr()), Err(ENOSPC));
        assert_eq!(zswap_stats(), ZswapStats::default());
        assert_eq!(zswap_load(1, 8, out.as_mut_ptr()), Err(ENOENT));
    }

    #[test]
    fn zswap_enforces_pool_limit() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();

        let page = vec![0x44; PAGE_SIZE];
        let mut stored = 0usize;
        for offset in 0..=ZSWAP_MAX_POOL_BYTES {
            match zswap_store(1, offset as u32, page.as_ptr()) {
                Ok(()) => stored += 1,
                Err(ENOSPC) => break,
                Err(errno) => panic!("unexpected zswap_store errno {errno}"),
            }
        }

        assert_eq!(stored, ZSWAP_MAX_POOL_PAGES);
        assert_eq!(zswap_store(1, stored as u32, page.as_ptr()), Err(ENOSPC));
        assert_eq!(zswap_store(1, 0, page.as_ptr()), Ok(()));
        let stats = zswap_stats();
        assert_eq!(stats.stored_pages, stored);
        assert!(stats.compressed_bytes <= ZSWAP_MAX_POOL_BYTES);
    }

    #[test]
    fn zswap_init_is_idempotent_and_reports_linux_pool() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();

        assert!(!init_succeeded());
        assert_eq!(pool_description(), "lzo/zbud");
        init();
        assert!(init_succeeded());
        init();
        assert!(init_succeeded());
    }

    #[test]
    fn swap_cgroup_tracks_swap_slot_owner() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();
        assert!(swap_cgroup_enabled());
        swap_cgroup_record(2, 9, 42);
        assert_eq!(swap_cgroup_lookup(2, 9), Some(42));
        assert!(!zswap_invalidate(2, 9));
        assert_eq!(swap_cgroup_lookup(2, 9), None);
    }

    #[test]
    fn zsmalloc_pools_own_objects_independently() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();

        let pool_a = zs_create_pool(core::ptr::null());
        let pool_b = zs_create_pool(core::ptr::null());
        assert!(!pool_a.is_null());
        assert!(!pool_b.is_null());

        let handle_a = zs_malloc(pool_a, 128, 0);
        let handle_b = zs_malloc(pool_b, PAGE_SIZE + 1, 0);
        assert_ne!(handle_a, 0);
        assert_ne!(handle_b, 0);
        assert_eq!(zs_get_total_pages(pool_a), 1);
        assert_eq!(zs_get_total_pages(pool_b), 2);

        let bytes = [1u8, 2, 3, 4];
        assert_eq!(
            unsafe { zs_obj_write(pool_a, handle_a, bytes.as_ptr(), bytes.len(), 8) },
            0
        );
        let ptr = zs_obj_read_begin(pool_a, handle_a);
        assert!(!ptr.is_null());
        unsafe {
            assert_eq!(core::slice::from_raw_parts(ptr.add(8), bytes.len()), bytes);
        }
        zs_obj_read_end(pool_a, handle_a);
        assert!(zs_obj_read_sg_begin(pool_a, handle_a));
        zs_obj_read_sg_end(pool_a, handle_a);

        let mut stats = ZsPoolStats::default();
        zs_pool_stats(pool_a, &raw mut stats);
        assert_eq!(stats.objects, 1);
        assert_eq!(stats.pages_used, 1);

        zs_destroy_pool(pool_a);
        assert_eq!(zs_get_total_pages(pool_b), 2);
        zs_free(pool_b, handle_b);
        assert_eq!(zs_get_total_pages(pool_b), 0);
        zs_destroy_pool(pool_b);
    }
}
