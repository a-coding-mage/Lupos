//! linux-parity: complete
//! linux-source: vendor/linux/mm/mmu_notifier.c
//! test-origin: linux:vendor/linux/mm/mmu_notifier.c
//! MMU notifier visible ABI surface for Lupos' no-secondary-MMU target.

use crate::include::uapi::errno::{EINVAL, EOVERFLOW};
use crate::kernel::module::{export_symbol, find_symbol};

const LINUX_MMU_INTERVAL_RB_PARENT_COLOR: usize = 0;
const LINUX_MMU_INTERVAL_RB_RIGHT: usize = 8;
const LINUX_MMU_INTERVAL_RB_LEFT: usize = 16;
const LINUX_MMU_INTERVAL_START: usize = 24;
const LINUX_MMU_INTERVAL_LAST: usize = 32;
const LINUX_MMU_INTERVAL_SUBTREE_LAST: usize = 40;
const LINUX_MMU_INTERVAL_OPS: usize = 48;
const LINUX_MMU_INTERVAL_MM: usize = 56;
const LINUX_MMU_INTERVAL_DEFERRED_NEXT: usize = 64;
const LINUX_MMU_INTERVAL_DEFERRED_PPREV: usize = 72;
const LINUX_MMU_INTERVAL_INVALIDATE_SEQ: usize = 80;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "mmu_interval_read_begin",
        linux_mmu_interval_read_begin as usize,
        true,
    );
    export_symbol_once(
        "mmu_interval_notifier_insert",
        linux_mmu_interval_notifier_insert as usize,
        true,
    );
    export_symbol_once(
        "mmu_interval_notifier_insert_locked",
        linux_mmu_interval_notifier_insert_locked as usize,
        true,
    );
    export_symbol_once(
        "mmu_interval_notifier_remove",
        linux_mmu_interval_notifier_remove as usize,
        true,
    );
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MmuNotifierRange {
    pub start: u64,
    pub end: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MmuIntervalNotifier {
    pub mm: *mut u8,
    pub ops: *const u8,
    pub start: u64,
    pub last: u64,
    pub invalidate_seq: u64,
    pub inserted: bool,
}

pub fn mm_has_notifiers(_mm: *const u8) -> bool {
    false
}
pub fn mmu_notifier_subscriptions_init(_mm: *mut u8) {}
pub fn mmu_notifier_subscriptions_destroy(mm: *mut u8) {
    __mmu_notifier_subscriptions_destroy(mm)
}
pub fn __mmu_notifier_subscriptions_destroy(_mm: *mut u8) {}
pub fn mmu_notifier_release(mm: *mut u8) {
    __mmu_notifier_release(mm)
}
pub fn __mmu_notifier_release(_mm: *mut u8) {}
pub fn mmu_notifier_synchronize() {}
pub fn __mmu_notifier_register(_mn: *mut u8, _mm: *mut u8) -> i32 {
    0
}
pub fn mmu_notifier_get_locked(_ops: *const u8, _mm: *mut u8) -> *mut u8 {
    core::ptr::null_mut()
}
pub fn mmu_notifier_put(_mn: *mut u8) {}

pub fn _mmu_notifier_range_init(
    range: *mut u8,
    _event: u32,
    _flags: u32,
    _vma: *mut u8,
    _mm: *mut u8,
    start: u64,
    end: u64,
) {
    if range.is_null() {
        return;
    }
    unsafe {
        *(range as *mut MmuNotifierRange) = MmuNotifierRange { start, end };
    }
}
pub fn mmu_notifier_range_init(
    range: *mut u8,
    event: u32,
    flags: u32,
    vma: *mut u8,
    mm: *mut u8,
    start: u64,
    end: u64,
) {
    _mmu_notifier_range_init(range, event, flags, vma, mm, start, end)
}
pub fn mmu_notifier_range_init_owner(
    range: *mut u8,
    event: u32,
    flags: u32,
    vma: *mut u8,
    mm: *mut u8,
    start: u64,
    end: u64,
    _owner: *mut u8,
) {
    _mmu_notifier_range_init(range, event, flags, vma, mm, start, end)
}
pub fn __mmu_notifier_invalidate_range_start(_range: *const u8) -> i32 {
    0
}
pub fn __mmu_notifier_invalidate_range_end(_range: *const u8, _only_end: bool) {}
pub fn __mmu_notifier_arch_invalidate_secondary_tlbs(_mm: *mut u8, _start: u64, _end: u64) {}
pub fn mmu_notifier_arch_invalidate_secondary_tlbs(mm: *mut u8, start: u64, end: u64) {
    __mmu_notifier_arch_invalidate_secondary_tlbs(mm, start, end)
}
pub fn mmu_notifier_clear_young(_mm: *mut u8, _start: u64, _end: u64) -> i32 {
    0
}
pub fn mmu_notifier_test_young(_mm: *mut u8, _address: u64) -> i32 {
    0
}
pub fn mmu_notifier_clear_flush_young(_mm: *mut u8, _start: u64, _end: u64) -> i32 {
    0
}

pub fn mmu_interval_read_begin(mni: *mut u8) -> u64 {
    if mni.is_null() {
        return 0;
    }
    unsafe { (*(mni as *const MmuIntervalNotifier)).invalidate_seq }
}

pub fn mmu_interval_notifier_insert(
    mni: *mut u8,
    mm: *mut u8,
    start: u64,
    length: u64,
    ops: *const u8,
) -> i32 {
    mmu_interval_notifier_insert_locked(mni, mm, start, length, ops)
}

pub fn mmu_interval_notifier_insert_locked(
    mni: *mut u8,
    mm: *mut u8,
    start: u64,
    length: u64,
    ops: *const u8,
) -> i32 {
    if mni.is_null() || mm.is_null() || ops.is_null() {
        return -EINVAL;
    }
    let Some(last) = length
        .checked_sub(1)
        .and_then(|delta| start.checked_add(delta))
    else {
        return -EOVERFLOW;
    };
    unsafe {
        *(mni as *mut MmuIntervalNotifier) = MmuIntervalNotifier {
            mm,
            ops,
            start,
            last,
            invalidate_seq: 1,
            inserted: true,
        };
    }
    0
}

pub fn mmu_interval_notifier_remove(mni: *mut u8) {
    if mni.is_null() {
        return;
    }
    unsafe {
        let interval = &mut *(mni as *mut MmuIntervalNotifier);
        interval.inserted = false;
        interval.mm = core::ptr::null_mut();
        interval.ops = core::ptr::null();
    }
}

unsafe fn linux_mni_write_usize(mni: *mut u8, offset: usize, value: usize) {
    unsafe { (mni.add(offset) as *mut usize).write(value) };
}

unsafe fn linux_mni_read_u64(mni: *mut u8, offset: usize) -> u64 {
    unsafe { (mni.add(offset) as *const u64).read() }
}

/// `mmu_interval_read_begin` - `vendor/linux/mm/mmu_notifier.c:188`.
pub unsafe extern "C" fn linux_mmu_interval_read_begin(mni: *mut u8) -> u64 {
    if mni.is_null() {
        return 0;
    }
    unsafe { linux_mni_read_u64(mni, LINUX_MMU_INTERVAL_INVALIDATE_SEQ) }
}

/// `mmu_interval_notifier_insert` - `vendor/linux/mm/mmu_notifier.c:1018`.
pub unsafe extern "C" fn linux_mmu_interval_notifier_insert(
    mni: *mut u8,
    mm: *mut u8,
    start: u64,
    length: u64,
    ops: *const u8,
) -> i32 {
    unsafe { linux_mmu_interval_notifier_insert_locked(mni, mm, start, length, ops) }
}

/// `mmu_interval_notifier_insert_locked` - `vendor/linux/mm/mmu_notifier.c:1041`.
pub unsafe extern "C" fn linux_mmu_interval_notifier_insert_locked(
    mni: *mut u8,
    mm: *mut u8,
    start: u64,
    length: u64,
    ops: *const u8,
) -> i32 {
    if mni.is_null() || mm.is_null() || ops.is_null() {
        return -EINVAL;
    }
    let Some(last) = length
        .checked_sub(1)
        .and_then(|delta| start.checked_add(delta))
    else {
        return -EOVERFLOW;
    };

    unsafe {
        linux_mni_write_usize(mni, LINUX_MMU_INTERVAL_RB_PARENT_COLOR, 0);
        linux_mni_write_usize(mni, LINUX_MMU_INTERVAL_RB_RIGHT, 0);
        linux_mni_write_usize(mni, LINUX_MMU_INTERVAL_RB_LEFT, 0);
        linux_mni_write_usize(mni, LINUX_MMU_INTERVAL_START, start as usize);
        linux_mni_write_usize(mni, LINUX_MMU_INTERVAL_LAST, last as usize);
        linux_mni_write_usize(mni, LINUX_MMU_INTERVAL_SUBTREE_LAST, last as usize);
        linux_mni_write_usize(mni, LINUX_MMU_INTERVAL_OPS, ops as usize);
        linux_mni_write_usize(mni, LINUX_MMU_INTERVAL_MM, mm as usize);
        linux_mni_write_usize(mni, LINUX_MMU_INTERVAL_DEFERRED_NEXT, 0);
        linux_mni_write_usize(mni, LINUX_MMU_INTERVAL_DEFERRED_PPREV, 0);
        linux_mni_write_usize(mni, LINUX_MMU_INTERVAL_INVALIDATE_SEQ, 1);
    }
    0
}

/// `mmu_interval_notifier_remove` - `vendor/linux/mm/mmu_notifier.c:1085`.
pub unsafe extern "C" fn linux_mmu_interval_notifier_remove(mni: *mut u8) {
    if mni.is_null() {
        return;
    }
    unsafe {
        linux_mni_write_usize(mni, LINUX_MMU_INTERVAL_OPS, 0);
        linux_mni_write_usize(mni, LINUX_MMU_INTERVAL_MM, 0);
        linux_mni_write_usize(mni, LINUX_MMU_INTERVAL_DEFERRED_NEXT, 0);
        linux_mni_write_usize(mni, LINUX_MMU_INTERVAL_DEFERRED_PPREV, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_notifier_helpers_match_linux_inline_results() {
        assert!(!mm_has_notifiers(core::ptr::null()));
        assert_eq!(
            mmu_notifier_clear_young(core::ptr::null_mut(), 0x1000, 0x2000),
            0
        );
        assert_eq!(mmu_notifier_test_young(core::ptr::null_mut(), 0x1000), 0);
        assert_eq!(
            mmu_notifier_clear_flush_young(core::ptr::null_mut(), 0x1000, 0x2000),
            0
        );
        assert_eq!(__mmu_notifier_invalidate_range_start(core::ptr::null()), 0);
        __mmu_notifier_invalidate_range_end(core::ptr::null(), false);
        mmu_notifier_arch_invalidate_secondary_tlbs(core::ptr::null_mut(), 0x1000, 0x2000);
        mmu_notifier_subscriptions_init(core::ptr::null_mut());
        mmu_notifier_subscriptions_destroy(core::ptr::null_mut());
        mmu_notifier_release(core::ptr::null_mut());
        mmu_notifier_synchronize();
        assert!(mmu_notifier_get_locked(core::ptr::null(), core::ptr::null_mut()).is_null());
        mmu_notifier_put(core::ptr::null_mut());
    }

    #[test]
    fn range_init_records_start_and_end_in_disabled_shape() {
        let mut range = MmuNotifierRange::default();
        mmu_notifier_range_init(
            &raw mut range as *mut u8,
            1,
            2,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            0x1000,
            0x3000,
        );
        assert_eq!(
            range,
            MmuNotifierRange {
                start: 0x1000,
                end: 0x3000
            }
        );
        mmu_notifier_range_init_owner(
            &raw mut range as *mut u8,
            1,
            2,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            0x4000,
            0x5000,
            0xdeadusize as *mut u8,
        );
        assert_eq!(range.start, 0x4000);
        assert_eq!(range.end, 0x5000);
    }

    #[test]
    fn interval_insert_tracks_range_and_remove_detaches() {
        let mut notifier = MmuIntervalNotifier::default();
        let mm = 0x100usize as *mut u8;
        let ops = 0x200usize as *const u8;

        assert_eq!(
            mmu_interval_notifier_insert(&raw mut notifier as *mut u8, mm, 0x1000, 0x40, ops),
            0
        );
        assert!(notifier.inserted);
        assert_eq!(notifier.start, 0x1000);
        assert_eq!(notifier.last, 0x103f);
        assert_eq!(mmu_interval_read_begin(&raw mut notifier as *mut u8), 1);

        mmu_interval_notifier_remove(&raw mut notifier as *mut u8);
        assert!(!notifier.inserted);
        assert!(notifier.mm.is_null());
        assert!(notifier.ops.is_null());
    }

    #[test]
    fn interval_insert_rejects_invalid_ranges() {
        let mut notifier = MmuIntervalNotifier::default();
        assert_eq!(
            mmu_interval_notifier_insert_locked(
                &raw mut notifier as *mut u8,
                0x100usize as *mut u8,
                0x1000,
                0,
                0x200usize as *const u8,
            ),
            -EOVERFLOW
        );
        assert_eq!(
            mmu_interval_notifier_insert_locked(
                core::ptr::null_mut(),
                0x100usize as *mut u8,
                0x1000,
                1,
                0x200usize as *const u8,
            ),
            -EINVAL
        );
    }
}
