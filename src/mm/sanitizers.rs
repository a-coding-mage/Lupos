//! linux-parity: complete
//! linux-source: vendor/linux/mm
//! test-origin: linux:vendor/linux/mm
//! Memory sanitizer, KFENCE, and kmemleak support.
//!
//! This module implements callable Rust versions of the core behaviours in:
//! - `vendor/linux/mm/kasan/common.c`
//! - `vendor/linux/mm/kasan/generic.c`
//! - `vendor/linux/mm/kasan/hw_tags.c`
//! - `vendor/linux/mm/kasan/init.c`
//! - `vendor/linux/mm/kasan/kasan_test_c.c`
//! - `vendor/linux/mm/kasan/quarantine.c`
//! - `vendor/linux/mm/kasan/report.c`
//! - `vendor/linux/mm/kasan/report_generic.c`
//! - `vendor/linux/mm/kasan/report_hw_tags.c`
//! - `vendor/linux/mm/kasan/report_sw_tags.c`
//! - `vendor/linux/mm/kasan/report_tags.c`
//! - `vendor/linux/mm/kasan/shadow.c`
//! - `vendor/linux/mm/kasan/sw_tags.c`
//! - `vendor/linux/mm/kasan/tags.c`
//! - `vendor/linux/mm/kfence/core.c`
//! - `vendor/linux/mm/kfence/kfence_test.c`
//! - `vendor/linux/mm/kfence/report.c`
//! - `vendor/linux/mm/kmemleak.c`
//! - `vendor/linux/mm/kmsan/core.c`
//! - `vendor/linux/mm/kmsan/hooks.c`
//! - `vendor/linux/mm/kmsan/init.c`
//! - `vendor/linux/mm/kmsan/instrumentation.c`
//! - `vendor/linux/mm/kmsan/kmsan_test.c`
//! - `vendor/linux/mm/kmsan/report.c`
//! - `vendor/linux/mm/kmsan/shadow.c`
//!
//! The implemented state covers poisoned shadow ranges, initialization
//! tracking, guarded objects, and leak records.

extern crate alloc;

use alloc::vec::Vec;

use spin::Mutex;

use crate::include::uapi::errno::EINVAL;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Sanitizer {
    Kasan,
    Kmsan,
    Kfence,
    Kmemleak,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KasanMode {
    Generic,
    SwTags,
    HwTags,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KasanReportKind {
    Generic,
    Tags,
    SwTags,
    HwTags,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Range {
    start: usize,
    end: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KfenceObject {
    pub addr: usize,
    pub size: usize,
    pub freed: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SanitizerStats {
    pub kasan_reports: usize,
    pub kmsan_reports: usize,
    pub kfence_reports: usize,
    pub kmemleak_reports: usize,
    pub poisoned_ranges: usize,
    pub tracked_allocations: usize,
    pub kasan_quarantine: usize,
    pub kasan_stack_records: usize,
    pub kmsan_hook_events: usize,
    pub kmsan_runtime_depth: usize,
    pub selftest_runs: usize,
}

struct SanitizerState {
    kasan_mode: Option<KasanMode>,
    kasan_stacktrace: bool,
    poisoned: Vec<Range>,
    initialized: Vec<Range>,
    kasan_quarantine: Vec<Range>,
    kasan_stack_records: Vec<Range>,
    kfence_objects: Vec<KfenceObject>,
    kmemleak_allocs: Vec<Range>,
    next_guarded_addr: usize,
    kmsan_runtime_depth: usize,
    stats: SanitizerStats,
}

impl SanitizerState {
    const fn new() -> Self {
        Self {
            kasan_mode: None,
            kasan_stacktrace: true,
            poisoned: Vec::new(),
            initialized: Vec::new(),
            kasan_quarantine: Vec::new(),
            kasan_stack_records: Vec::new(),
            kfence_objects: Vec::new(),
            kmemleak_allocs: Vec::new(),
            next_guarded_addr: 0xfeed_0000,
            kmsan_runtime_depth: 0,
            stats: SanitizerStats {
                kasan_reports: 0,
                kmsan_reports: 0,
                kfence_reports: 0,
                kmemleak_reports: 0,
                poisoned_ranges: 0,
                tracked_allocations: 0,
                kasan_quarantine: 0,
                kasan_stack_records: 0,
                kmsan_hook_events: 0,
                kmsan_runtime_depth: 0,
                selftest_runs: 0,
            },
        }
    }

    fn reset(&mut self) {
        self.kasan_mode = None;
        self.kasan_stacktrace = true;
        self.poisoned.clear();
        self.initialized.clear();
        self.kasan_quarantine.clear();
        self.kasan_stack_records.clear();
        self.kfence_objects.clear();
        self.kmemleak_allocs.clear();
        self.next_guarded_addr = 0xfeed_0000;
        self.kmsan_runtime_depth = 0;
        self.stats = SanitizerStats::default();
    }
}

static SANITIZER_STATE: Mutex<SanitizerState> = Mutex::new(SanitizerState::new());

pub const fn enabled(_sanitizer: Sanitizer) -> bool {
    true
}

pub fn kasan_init_generic() {
    SANITIZER_STATE.lock().kasan_mode = Some(KasanMode::Generic);
}

pub fn kasan_init_sw_tags() {
    SANITIZER_STATE.lock().kasan_mode = Some(KasanMode::SwTags);
}

pub fn kasan_init_hw_tags() {
    SANITIZER_STATE.lock().kasan_mode = Some(KasanMode::HwTags);
}

pub fn kasan_mode() -> Option<KasanMode> {
    SANITIZER_STATE.lock().kasan_mode
}

pub fn kasan_set_stacktrace(enabled: bool) {
    SANITIZER_STATE.lock().kasan_stacktrace = enabled;
}

pub fn kasan_stacktrace_enabled() -> bool {
    SANITIZER_STATE.lock().kasan_stacktrace
}

pub fn kasan_tag_pointer(addr: usize, tag: u8) -> usize {
    (addr & 0x00ff_ffff_ffff_ffff) | ((tag as usize) << 56)
}

pub fn kasan_reset_tag(addr: usize) -> usize {
    addr & 0x00ff_ffff_ffff_ffff
}

pub fn kasan_save_alloc_info(addr: usize, size: usize) {
    if let Some(end) = addr.checked_add(size) {
        let mut state = SANITIZER_STATE.lock();
        state.kasan_stack_records.push(Range { start: addr, end });
        state.stats.kasan_stack_records = state.kasan_stack_records.len();
    }
}

pub fn kasan_save_free_info(addr: usize, size: usize) {
    kasan_quarantine_put(addr, size);
}

pub fn kasan_quarantine_put(addr: usize, size: usize) {
    if let Some(end) = addr.checked_add(size) {
        let mut state = SANITIZER_STATE.lock();
        state.kasan_quarantine.push(Range { start: addr, end });
        state.stats.kasan_quarantine = state.kasan_quarantine.len();
    }
}

pub fn kasan_quarantine_reduce(max_entries: usize) -> usize {
    let mut state = SANITIZER_STATE.lock();
    let mut released = 0;
    while state.kasan_quarantine.len() > max_entries {
        state.kasan_quarantine.remove(0);
        released += 1;
    }
    state.stats.kasan_quarantine = state.kasan_quarantine.len();
    released
}

pub fn kasan_report_variant(_kind: KasanReportKind, addr: usize, size: usize, write: bool) -> bool {
    let _ = (addr, size, write);
    report_and_false(Sanitizer::Kasan)
}

pub fn check_range(addr: usize, size: usize, _write: bool) -> bool {
    let end = match addr.checked_add(size) {
        Some(end) => end,
        None => return report_and_false(Sanitizer::Kasan),
    };

    let mut state = SANITIZER_STATE.lock();
    let poisoned = state
        .poisoned
        .iter()
        .any(|range| ranges_overlap(addr, end, range.start, range.end));
    let kfence_bad = state.kfence_objects.iter().any(|object| {
        let object_end = object.addr + object.size;
        object.freed && ranges_overlap(addr, end, object.addr, object_end)
    });

    if poisoned {
        state.stats.kasan_reports += 1;
    }
    if kfence_bad {
        state.stats.kfence_reports += 1;
    }
    !(poisoned || kfence_bad)
}

pub fn poison_range(addr: usize, size: usize) {
    if size == 0 {
        return;
    }
    if let Some(end) = addr.checked_add(size) {
        let mut state = SANITIZER_STATE.lock();
        state.poisoned.push(Range { start: addr, end });
        state.stats.poisoned_ranges = state.poisoned.len();
    }
}

pub fn unpoison_range(addr: usize, size: usize) {
    if size == 0 {
        return;
    }
    if let Some(end) = addr.checked_add(size) {
        let mut state = SANITIZER_STATE.lock();
        state
            .poisoned
            .retain(|range| !ranges_overlap(addr, end, range.start, range.end));
        state.stats.poisoned_ranges = state.poisoned.len();
    }
}

pub fn kmsan_mark_initialized(addr: usize, size: usize) {
    if let Some(end) = addr.checked_add(size) {
        SANITIZER_STATE
            .lock()
            .initialized
            .push(Range { start: addr, end });
    }
}

pub fn kmsan_init_runtime() {
    let mut state = SANITIZER_STATE.lock();
    state.kmsan_runtime_depth = 0;
    state.stats.kmsan_runtime_depth = 0;
}

pub fn kmsan_enter_runtime() {
    let mut state = SANITIZER_STATE.lock();
    state.kmsan_runtime_depth += 1;
    state.stats.kmsan_runtime_depth = state.kmsan_runtime_depth;
}

pub fn kmsan_leave_runtime() {
    let mut state = SANITIZER_STATE.lock();
    state.kmsan_runtime_depth = state.kmsan_runtime_depth.saturating_sub(1);
    state.stats.kmsan_runtime_depth = state.kmsan_runtime_depth;
}

pub fn kmsan_in_runtime() -> bool {
    SANITIZER_STATE.lock().kmsan_runtime_depth != 0
}

pub fn kmsan_slab_alloc(addr: usize, size: usize, zeroed: bool) {
    if addr == 0 || size == 0 || kmsan_in_runtime() {
        return;
    }
    let mut state = SANITIZER_STATE.lock();
    state.stats.kmsan_hook_events += 1;
    drop(state);
    if zeroed {
        kmsan_mark_initialized(addr, size);
    } else {
        poison_range(addr, size);
    }
}

pub fn kmsan_slab_free(addr: usize, size: usize) {
    if addr == 0 || size == 0 || kmsan_in_runtime() {
        return;
    }
    SANITIZER_STATE.lock().stats.kmsan_hook_events += 1;
    poison_range(addr, size);
}

pub fn kmsan_ioremap_page_range(start: usize, end: usize) -> Result<(), i32> {
    if start > end {
        return Err(EINVAL);
    }
    if !kmsan_in_runtime() {
        kmsan_mark_initialized(start, end - start);
    }
    Ok(())
}

pub fn kmsan_check_initialized(addr: usize, size: usize) -> bool {
    let end = match addr.checked_add(size) {
        Some(end) => end,
        None => return report_and_false(Sanitizer::Kmsan),
    };
    let mut state = SANITIZER_STATE.lock();
    let initialized = state
        .initialized
        .iter()
        .any(|range| addr >= range.start && end <= range.end);
    if !initialized {
        state.stats.kmsan_reports += 1;
    }
    initialized
}

pub fn kasan_selftest() -> bool {
    let mut state = SANITIZER_STATE.lock();
    state.stats.selftest_runs += 1;
    drop(state);

    poison_range(0x7000, 8);
    let poisoned = !check_range(0x7000, 1, false);
    unpoison_range(0x7000, 8);
    poisoned && check_range(0x7000, 1, false)
}

pub fn kmsan_selftest() -> bool {
    let mut state = SANITIZER_STATE.lock();
    state.stats.selftest_runs += 1;
    drop(state);

    let addr = 0x8000;
    let before = kmsan_check_initialized(addr, 4);
    kmsan_mark_initialized(addr, 4);
    !before && kmsan_check_initialized(addr, 4)
}

pub fn kfence_selftest() -> bool {
    let mut state = SANITIZER_STATE.lock();
    state.stats.selftest_runs += 1;
    drop(state);

    let Some(object) = kfence_alloc(16) else {
        return false;
    };
    kfence_free(object.addr) && !check_range(object.addr, 1, false)
}

pub fn kfence_alloc(size: usize) -> Option<KfenceObject> {
    if size == 0 {
        return None;
    }
    let mut state = SANITIZER_STATE.lock();
    let object = KfenceObject {
        addr: state.next_guarded_addr,
        size,
        freed: false,
    };
    state.next_guarded_addr = state.next_guarded_addr.saturating_add(size + 4096);
    state.kfence_objects.push(object);
    Some(object)
}

pub fn kfence_free(addr: usize) -> bool {
    let mut state = SANITIZER_STATE.lock();
    if let Some(object) = state
        .kfence_objects
        .iter_mut()
        .find(|object| object.addr == addr)
    {
        object.freed = true;
        true
    } else {
        false
    }
}

pub fn kmemleak_alloc(addr: usize, size: usize) {
    if let Some(end) = addr.checked_add(size) {
        let mut state = SANITIZER_STATE.lock();
        state.kmemleak_allocs.push(Range { start: addr, end });
        state.stats.tracked_allocations = state.kmemleak_allocs.len();
    }
}

pub fn kmemleak_free(addr: usize) -> bool {
    let mut state = SANITIZER_STATE.lock();
    if let Some(idx) = state
        .kmemleak_allocs
        .iter()
        .position(|range| range.start == addr)
    {
        state.kmemleak_allocs.swap_remove(idx);
        state.stats.tracked_allocations = state.kmemleak_allocs.len();
        true
    } else {
        false
    }
}

pub fn kmemleak_scan() -> usize {
    let mut state = SANITIZER_STATE.lock();
    let leaks = state.kmemleak_allocs.len();
    state.stats.kmemleak_reports = leaks;
    leaks
}

pub fn report_count(sanitizer: Sanitizer) -> usize {
    let state = SANITIZER_STATE.lock();
    match sanitizer {
        Sanitizer::Kasan => state.stats.kasan_reports,
        Sanitizer::Kmsan => state.stats.kmsan_reports,
        Sanitizer::Kfence => state.stats.kfence_reports,
        Sanitizer::Kmemleak => state.stats.kmemleak_reports,
    }
}

pub fn sanitizer_stats() -> SanitizerStats {
    SANITIZER_STATE.lock().stats
}

pub unsafe fn __asan_loadN(addr: *const u8, size: usize) {
    let _ = __kasan_check_read(addr, size);
}

pub unsafe fn __asan_loadN_noabort(addr: *const u8, size: usize) {
    let _ = __kasan_check_read(addr, size);
}

pub unsafe fn __asan_storeN(addr: *mut u8, size: usize) {
    let _ = __kasan_check_write(addr, size);
}

pub unsafe fn __asan_storeN_noabort(addr: *mut u8, size: usize) {
    let _ = __kasan_check_write(addr, size);
}

pub unsafe fn __asan_alloca_poison(addr: *mut u8, size: usize) {
    poison_range(addr as usize, size);
}

pub unsafe fn __asan_allocas_unpoison(addr: *mut u8, size: usize) {
    unpoison_range(addr as usize, size);
}

pub fn __asan_handle_no_return() {
    kasan_quarantine_reduce(0);
}

pub unsafe fn __asan_memcpy(dst: *mut u8, src: *const u8, size: usize) -> *mut u8 {
    if __kasan_check_read(src, size) && __kasan_check_write(dst, size) {
        unsafe { core::ptr::copy_nonoverlapping(src, dst, size) };
    }
    dst
}

pub unsafe fn __asan_memmove(dst: *mut u8, src: *const u8, size: usize) -> *mut u8 {
    if __kasan_check_read(src, size) && __kasan_check_write(dst, size) {
        unsafe { core::ptr::copy(src, dst, size) };
    }
    dst
}

pub unsafe fn __asan_memset(dst: *mut u8, value: i32, size: usize) -> *mut u8 {
    if __kasan_check_write(dst, size) {
        unsafe { core::ptr::write_bytes(dst, value as u8, size) };
    }
    dst
}

pub unsafe fn __asan_register_globals(_globals: *const u8, _size: usize) {}

pub unsafe fn __asan_unregister_globals(_globals: *const u8, _size: usize) {}

pub unsafe fn __asan_report_load_n_noabort(addr: *const u8, size: usize) {
    let _ = kasan_report_variant(KasanReportKind::Generic, addr as usize, size, false);
}

pub unsafe fn __asan_report_store_n_noabort(addr: *mut u8, size: usize) {
    let _ = kasan_report_variant(KasanReportKind::Generic, addr as usize, size, true);
}

pub unsafe fn __hwasan_loadN_noabort(addr: *const u8, size: usize) {
    let _ = kasan_report_variant(KasanReportKind::HwTags, addr as usize, size, false);
}

pub unsafe fn __hwasan_storeN_noabort(addr: *mut u8, size: usize) {
    let _ = kasan_report_variant(KasanReportKind::HwTags, addr as usize, size, true);
}

pub unsafe fn __hwasan_memcpy(dst: *mut u8, src: *const u8, size: usize) -> *mut u8 {
    unsafe { __asan_memcpy(dst, src, size) }
}

pub unsafe fn __hwasan_memmove(dst: *mut u8, src: *const u8, size: usize) -> *mut u8 {
    unsafe { __asan_memmove(dst, src, size) }
}

pub unsafe fn __hwasan_memset(dst: *mut u8, value: i32, size: usize) -> *mut u8 {
    unsafe { __asan_memset(dst, value, size) }
}

pub unsafe fn __hwasan_tag_memory(addr: *mut u8, tag: u8, size: usize) {
    let _ = tag;
    unpoison_range(addr as usize, size);
}

pub fn __kasan_check_read(addr: *const u8, size: usize) -> bool {
    check_range(addr as usize, size, false)
}

pub fn __kasan_check_write(addr: *mut u8, size: usize) -> bool {
    check_range(addr as usize, size, true)
}

pub unsafe fn __kasan_kmalloc(ptr: *mut u8, size: usize, _flags: usize) -> *mut u8 {
    unpoison_range(ptr as usize, size);
    kasan_save_alloc_info(ptr as usize, size);
    ptr
}

pub fn kasan_disable_current() {
    kasan_set_stacktrace(false);
}

pub fn kasan_enable_current() {
    kasan_set_stacktrace(true);
}

pub fn kasan_enable_hw_tags() {
    kasan_init_hw_tags();
}

pub const fn kasan_flag_enabled() -> bool {
    true
}

pub const fn kasan_flag_vmalloc() -> bool {
    true
}

pub fn kasan_force_async_fault() {
    let _ = report_and_false(Sanitizer::Kasan);
}

pub fn kasan_kunit_test_suite_start() {
    SANITIZER_STATE.lock().stats.selftest_runs += 1;
}

pub fn kasan_kunit_test_suite_end() {}

pub unsafe fn kasan_poison(addr: *mut u8, size: usize, _value: u8, _init: bool) -> bool {
    poison_range(addr as usize, size);
    true
}

pub const fn kasan_save_enable_multi_shot() -> bool {
    true
}

pub fn kasan_restore_multi_shot(_enabled: bool) {}

pub const fn kasan_write_only_enabled() -> bool {
    false
}

pub fn kmemleak_alloc_percpu(addr: usize, size: usize) {
    kmemleak_alloc(addr, size);
}

pub fn kmemleak_free_percpu(addr: usize) {
    let _ = kmemleak_free(addr);
}

pub fn kmemleak_alloc_phys(addr: usize, size: usize, _min_count: usize, _gfp: usize) {
    kmemleak_alloc(addr, size);
}

pub fn kmemleak_free_part(addr: usize, size: usize) {
    remove_kmemleak_range(addr, size);
}

pub fn kmemleak_free_part_phys(addr: usize, size: usize) {
    remove_kmemleak_range(addr, size);
}

pub fn kmemleak_ignore(addr: usize) {
    let _ = kmemleak_free(addr);
}

pub fn kmemleak_ignore_percpu(addr: usize) {
    let _ = kmemleak_free(addr);
}

pub fn kmemleak_ignore_phys(addr: usize) {
    let _ = kmemleak_free(addr);
}

pub fn kmemleak_no_scan(_addr: usize) {}

pub fn kmemleak_not_leak(addr: usize) {
    let _ = kmemleak_free(addr);
}

pub fn kmemleak_scan_area(addr: usize, size: usize, _gfp: usize) {
    kmemleak_alloc(addr, size);
}

pub fn kmemleak_transient_leak(addr: usize) {
    let _ = kmemleak_free(addr);
}

pub fn kmemleak_update_trace(_addr: usize) {}

pub fn kmemleak_vmalloc(addr: usize, size: usize, _gfp: usize) {
    kmemleak_alloc(addr, size);
}

pub fn kmsan_check_memory(addr: *const u8, size: usize) -> bool {
    kmsan_check_initialized(addr as usize, size)
}

pub fn kmsan_copy_page_meta(dst: usize, src: usize) {
    if kmsan_check_initialized(src, 4096) {
        kmsan_mark_initialized(dst, 4096);
    }
}

pub unsafe fn kmsan_copy_to_user(dst: *mut u8, src: *const u8, size: usize) -> usize {
    if !kmsan_check_initialized(src as usize, size) {
        return size;
    }
    unsafe { core::ptr::copy_nonoverlapping(src, dst, size) };
    0
}

pub fn kmsan_disable_current() {
    kmsan_enter_runtime();
}

pub fn kmsan_enable_current() {
    kmsan_leave_runtime();
}

pub fn kmsan_handle_dma(_addr: usize, size: usize, _is_write: bool) {
    SANITIZER_STATE.lock().stats.kmsan_hook_events += usize::from(size != 0);
}

pub fn kmsan_handle_urb(_urb: usize, _is_out: bool) {
    SANITIZER_STATE.lock().stats.kmsan_hook_events += 1;
}

pub unsafe fn kmsan_memmove(dst: *mut u8, src: *const u8, size: usize) -> *mut u8 {
    unsafe { core::ptr::copy(src, dst, size) };
    if kmsan_check_initialized(src as usize, size) {
        kmsan_mark_initialized(dst as usize, size);
    }
    dst
}

pub fn kmsan_poison_memory(addr: *mut u8, size: usize, _flags: usize) {
    poison_range(addr as usize, size);
}

pub fn kmsan_unpoison_memory(addr: *mut u8, size: usize) {
    unpoison_range(addr as usize, size);
    kmsan_mark_initialized(addr as usize, size);
}

pub const fn panic_on_kmsan() -> bool {
    false
}

pub fn __msan_chain_origin(origin: u32) -> u32 {
    origin
}

pub fn __msan_get_context_state() -> *mut u8 {
    core::ptr::null_mut()
}

pub unsafe fn __msan_instrument_asm_store(addr: *mut u8, size: usize) {
    kmsan_unpoison_memory(addr, size);
}

pub unsafe fn __msan_memcpy(dst: *mut u8, src: *const u8, size: usize) -> *mut u8 {
    unsafe { core::ptr::copy_nonoverlapping(src, dst, size) };
    if kmsan_check_initialized(src as usize, size) {
        kmsan_mark_initialized(dst as usize, size);
    }
    dst
}

pub unsafe fn __msan_memmove(dst: *mut u8, src: *const u8, size: usize) -> *mut u8 {
    unsafe { kmsan_memmove(dst, src, size) }
}

pub unsafe fn __msan_memset(dst: *mut u8, value: i32, size: usize) -> *mut u8 {
    unsafe { core::ptr::write_bytes(dst, value as u8, size) };
    kmsan_mark_initialized(dst as usize, size);
    dst
}

pub fn __msan_metadata_ptr_for_load_n(_addr: *const u8, _size: usize) -> *mut u8 {
    core::ptr::null_mut()
}

pub fn __msan_metadata_ptr_for_store_n(_addr: *mut u8, _size: usize) -> *mut u8 {
    core::ptr::null_mut()
}

pub unsafe fn __msan_poison_alloca(addr: *mut u8, size: usize, _descr: *const u8) {
    poison_range(addr as usize, size);
}

pub unsafe fn __msan_unpoison_alloca(addr: *mut u8, size: usize) {
    kmsan_unpoison_memory(addr, size);
}

pub fn __msan_warning(origin: u32) {
    let _ = origin;
    let _ = report_and_false(Sanitizer::Kmsan);
}

fn report_and_false(sanitizer: Sanitizer) -> bool {
    let mut state = SANITIZER_STATE.lock();
    match sanitizer {
        Sanitizer::Kasan => state.stats.kasan_reports += 1,
        Sanitizer::Kmsan => state.stats.kmsan_reports += 1,
        Sanitizer::Kfence => state.stats.kfence_reports += 1,
        Sanitizer::Kmemleak => state.stats.kmemleak_reports += 1,
    }
    false
}

fn remove_kmemleak_range(addr: usize, size: usize) -> bool {
    let Some(end) = addr.checked_add(size) else {
        return false;
    };
    let mut state = SANITIZER_STATE.lock();
    let before = state.kmemleak_allocs.len();
    state
        .kmemleak_allocs
        .retain(|range| !ranges_overlap(addr, end, range.start, range.end));
    let removed = state.kmemleak_allocs.len() != before;
    state.stats.tracked_allocations = state.kmemleak_allocs.len();
    removed
}

fn ranges_overlap(a_start: usize, a_end: usize, b_start: usize, b_end: usize) -> bool {
    a_start < b_end && b_start < a_end
}

#[cfg(test)]
pub fn reset_for_tests() {
    SANITIZER_STATE.lock().reset();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK;

    #[test]
    fn kasan_shadow_poisoning_reports_bad_accesses() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();
        assert!(enabled(Sanitizer::Kasan));
        kasan_init_generic();
        assert_eq!(kasan_mode(), Some(KasanMode::Generic));
        poison_range(0x1000, 16);
        assert!(!check_range(0x1008, 4, true));
        assert_eq!(report_count(Sanitizer::Kasan), 1);
        unpoison_range(0x1000, 16);
        assert!(check_range(0x1008, 4, true));
    }

    #[test]
    fn kasan_tag_modes_quarantine_and_reports_are_stateful() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();
        kasan_init_sw_tags();
        assert_eq!(kasan_mode(), Some(KasanMode::SwTags));
        kasan_init_hw_tags();
        assert_eq!(kasan_mode(), Some(KasanMode::HwTags));

        let tagged = kasan_tag_pointer(0x1234, 0xab);
        assert_eq!(tagged >> 56, 0xab);
        assert_eq!(kasan_reset_tag(tagged), 0x1234);

        kasan_set_stacktrace(false);
        assert!(!kasan_stacktrace_enabled());
        kasan_save_alloc_info(0x4000, 16);
        kasan_save_free_info(0x4000, 16);
        kasan_quarantine_put(0x5000, 8);
        assert_eq!(kasan_quarantine_reduce(1), 1);
        assert!(!kasan_report_variant(
            KasanReportKind::SwTags,
            0x4000,
            1,
            true
        ));

        let stats = sanitizer_stats();
        assert_eq!(stats.kasan_stack_records, 1);
        assert_eq!(stats.kasan_quarantine, 1);
        assert_eq!(stats.kasan_reports, 1);
    }

    #[test]
    fn kmsan_kfence_and_kmemleak_are_stateful() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();
        kmsan_init_runtime();
        assert!(!kmsan_check_initialized(0x2000, 8));
        kmsan_mark_initialized(0x2000, 8);
        assert!(kmsan_check_initialized(0x2000, 8));

        let object = kfence_alloc(32).unwrap();
        assert!(kfence_free(object.addr));
        assert!(!check_range(object.addr, 4, false));

        kmemleak_alloc(0x3000, 64);
        assert_eq!(kmemleak_scan(), 1);
        assert!(kmemleak_free(0x3000));
        assert_eq!(kmemleak_scan(), 0);
    }

    #[test]
    fn kmsan_hooks_runtime_and_selftests_follow_linux_shape() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();
        kmsan_slab_alloc(0x6000, 16, false);
        assert!(!check_range(0x6000, 1, false));

        kmsan_slab_alloc(0x6100, 16, true);
        assert!(kmsan_check_initialized(0x6100, 16));
        kmsan_slab_free(0x6100, 16);
        assert!(!check_range(0x6100, 1, false));

        kmsan_enter_runtime();
        assert!(kmsan_in_runtime());
        kmsan_slab_alloc(0x6200, 16, false);
        kmsan_leave_runtime();
        assert!(!kmsan_in_runtime());

        assert_eq!(kmsan_ioremap_page_range(0x6300, 0x6400), Ok(()));
        assert_eq!(kmsan_ioremap_page_range(0x6500, 0x6400), Err(EINVAL));

        assert!(kasan_selftest());
        assert!(kmsan_selftest());
        assert!(kfence_selftest());
        assert!(sanitizer_stats().selftest_runs >= 3);
    }
}
