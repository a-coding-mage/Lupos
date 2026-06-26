//! linux-parity: complete
//! linux-source: vendor/linux/mm/shrinker.c
//! test-origin: linux:vendor/linux/mm/shrinker.c
/// Shrinker registry and slab-cache reclaim hooks.
///
/// This is the Rust analogue of Linux's shrinker API, enough for upcoming
/// dentry/inode caches to register reclaim callbacks cleanly.
///
/// Ref: Linux `include/linux/shrinker.h`
///      Linux `mm/shrinker.c`
extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicIsize, Ordering};

use spin::Mutex;

use super::page_flags::GfpFlags;

pub const SHRINK_EMPTY: usize = usize::MAX;
pub const SHRINK_STOP: usize = usize::MAX - 1;
pub const DEFAULT_SEEKS: usize = 2;
const SHRINK_BATCH: usize = 128;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShrinkControl {
    pub nr_to_scan: usize,
    pub nr_scanned: usize,
    pub gfp_mask: GfpFlags,
    pub nid: i32,
}

#[repr(C)]
pub struct Shrinker {
    pub name: &'static str,
    pub count_objects: fn(&Shrinker, &ShrinkControl) -> usize,
    pub scan_objects: fn(&Shrinker, &mut ShrinkControl) -> usize,
    pub seeks: usize,
    pub batch: usize,
    pub id: isize,
    pub nr_deferred: AtomicIsize,
    pub private_data: usize,
}

impl Shrinker {
    pub fn new(
        name: &'static str,
        count_objects: fn(&Shrinker, &ShrinkControl) -> usize,
        scan_objects: fn(&Shrinker, &mut ShrinkControl) -> usize,
    ) -> Self {
        Self {
            name,
            count_objects,
            scan_objects,
            seeks: DEFAULT_SEEKS,
            batch: 0,
            id: -1,
            nr_deferred: AtomicIsize::new(0),
            private_data: 0,
        }
    }
}

struct ShrinkerRegistry {
    next_id: isize,
    shrinkers: Vec<usize>,
}

impl ShrinkerRegistry {
    const fn new() -> Self {
        Self {
            next_id: 0,
            shrinkers: Vec::new(),
        }
    }
}

static SHRINKERS: Mutex<ShrinkerRegistry> = Mutex::new(ShrinkerRegistry::new());

pub unsafe fn register_shrinker(shrinker: *mut Shrinker) -> isize {
    if shrinker.is_null() {
        return -1;
    }

    let mut registry = SHRINKERS.lock();
    let key = shrinker as usize;
    if registry
        .shrinkers
        .iter()
        .copied()
        .any(|registered| registered == key)
    {
        return unsafe { (*shrinker).id };
    }

    let id = registry.next_id;
    registry.next_id += 1;
    registry.shrinkers.push(key);
    unsafe {
        (*shrinker).id = id;
    }
    id
}

pub unsafe fn unregister_shrinker(shrinker: *mut Shrinker) {
    if shrinker.is_null() {
        return;
    }

    let mut registry = SHRINKERS.lock();
    if let Some(idx) = registry
        .shrinkers
        .iter()
        .position(|&registered| registered == shrinker as usize)
    {
        registry.shrinkers.swap_remove(idx);
    }
    unsafe {
        (*shrinker).id = -1;
        (*shrinker).nr_deferred.store(0, Ordering::Relaxed);
    }
}

fn do_shrink_slab(shrinkctl: &mut ShrinkControl, shrinker: &Shrinker, priority: usize) -> usize {
    let freeable = (shrinker.count_objects)(shrinker, shrinkctl);
    if freeable == 0 || freeable == SHRINK_EMPTY {
        return 0;
    }

    let batch_size = if shrinker.batch != 0 {
        shrinker.batch
    } else {
        SHRINK_BATCH
    };
    let nr = shrinker.nr_deferred.swap(0, Ordering::AcqRel).max(0) as usize;

    let delta = if shrinker.seeks != 0 {
        (((freeable >> priority) * 4) / shrinker.seeks).min(2 * freeable)
    } else {
        freeable / 2
    };

    let mut total_scan = (nr >> priority).saturating_add(delta);
    total_scan = total_scan.min(2 * freeable);

    let mut freed = 0usize;
    let mut scanned = 0usize;

    while total_scan >= batch_size || total_scan >= freeable {
        let nr_to_scan = batch_size.min(total_scan);
        shrinkctl.nr_to_scan = nr_to_scan;
        shrinkctl.nr_scanned = nr_to_scan;

        let ret = (shrinker.scan_objects)(shrinker, shrinkctl);
        if ret == SHRINK_STOP {
            break;
        }

        // Mirror the kernel's "actual progress" contract and bail out when a
        // shrinker can't scan anything further, otherwise total_scan never
        // decreases and reclaim can livelock.
        if shrinkctl.nr_scanned == 0 {
            break;
        }

        freed += ret;
        total_scan = total_scan.saturating_sub(shrinkctl.nr_scanned);
        scanned += shrinkctl.nr_scanned;
    }

    let next_deferred = nr
        .saturating_add(delta)
        .saturating_sub(scanned)
        .min(2 * freeable);
    shrinker
        .nr_deferred
        .store(next_deferred as isize, Ordering::Release);

    freed
}

pub fn shrink_slab(gfp_mask: GfpFlags, priority: usize) -> usize {
    let shrinkers = SHRINKERS.lock().shrinkers.clone();
    let mut freed = 0usize;

    for shrinker in shrinkers {
        let shrinker = shrinker as *mut Shrinker;
        if shrinker.is_null() {
            continue;
        }

        let mut sc = ShrinkControl {
            gfp_mask,
            nid: 0,
            nr_to_scan: 0,
            nr_scanned: 0,
        };
        freed += do_shrink_slab(&mut sc, unsafe { &*shrinker }, priority);
    }

    freed
}

#[cfg(test)]
pub fn reset_shrinker_state_for_test() {
    *SHRINKERS.lock() = ShrinkerRegistry::new();
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    extern crate std;

    use alloc::boxed::Box;
    use core::sync::atomic::AtomicUsize;

    use super::*;
    use crate::mm::buddy::reset_buddy_state_for_test;
    use crate::mm::page_flags::GFP_KERNEL;
    use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK;

    struct DummyState {
        freeable: AtomicUsize,
        freed: AtomicUsize,
    }

    fn count_objects(shrinker: &Shrinker, _sc: &ShrinkControl) -> usize {
        let state = unsafe { &*(shrinker.private_data as *const DummyState) };
        state
            .freeable
            .load(Ordering::Relaxed)
            .saturating_sub(state.freed.load(Ordering::Relaxed))
    }

    fn scan_objects(shrinker: &Shrinker, sc: &mut ShrinkControl) -> usize {
        let state = unsafe { &*(shrinker.private_data as *const DummyState) };
        let remaining = state
            .freeable
            .load(Ordering::Relaxed)
            .saturating_sub(state.freed.load(Ordering::Relaxed));
        let actual = (sc.nr_to_scan / 2).max(1).min(remaining);
        sc.nr_scanned = actual;
        state.freed.fetch_add(actual, Ordering::Relaxed);
        actual
    }

    fn test_guard() -> std::sync::MutexGuard<'static, ()> {
        let guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        reset_buddy_state_for_test();
        reset_shrinker_state_for_test();
        guard
    }

    #[test]
    fn shrink_slab_carries_deferred_work() {
        let _guard = test_guard();

        let state = Box::new(DummyState {
            freeable: AtomicUsize::new(256),
            freed: AtomicUsize::new(0),
        });
        let state_ptr = Box::into_raw(state);

        let mut shrinker = Box::new(Shrinker::new("dummy", count_objects, scan_objects));
        shrinker.batch = 64;
        shrinker.seeks = 4;
        shrinker.private_data = state_ptr as usize;
        let shrinker_ptr = shrinker.as_mut() as *mut Shrinker;

        unsafe { register_shrinker(shrinker_ptr) };
        let first = shrink_slab(GFP_KERNEL, 0);
        assert!(first > 0);
        assert!(shrinker.nr_deferred.load(Ordering::Relaxed) > 0);

        let second = shrink_slab(GFP_KERNEL, 0);
        assert!(second > 0);
        assert!(unsafe { &*state_ptr }.freed.load(Ordering::Relaxed) >= first + second);

        unsafe { unregister_shrinker(shrinker_ptr) };
        unsafe {
            drop(Box::from_raw(state_ptr));
        }
    }
}
