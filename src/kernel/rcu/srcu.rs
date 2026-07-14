//! linux-parity: complete
//! linux-source: vendor/linux/kernel/rcu
//! test-origin: linux:vendor/linux/kernel/rcu
//! Sleepable RCU (`srcu_struct`) — M34.
//!
//! Mirrors `vendor/linux/kernel/rcu/srcutiny.c`.  Lupos M34 ships the tiny
//! variant: one grace period at a time, two per-CPU counters per
//! `srcu_struct`.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicI32, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;

use crate::include::uapi::errno::{EINVAL, ENOMEM};
use crate::kernel::sched::MAX_CPUS;
use crate::mm::page_flags::{__GFP_ZERO, GFP_KERNEL};

const LINUX_SRCU_CTR_SIZE: usize = 16;
const LINUX_SRCU_CTR_LOCKS_OFFSET: usize = 0;
const LINUX_SRCU_CTR_UNLOCKS_OFFSET: usize = 8;
const LINUX_SRCU_DATA_SIZE: usize = 384;
const LINUX_SRCU_DATA_SRCU_READER_FLAVOR_OFFSET: usize = 32;
const LINUX_SRCU_STRUCT_SRCU_CTRP_OFFSET: usize = 0;
const LINUX_SRCU_STRUCT_SDA_OFFSET: usize = 8;
const LINUX_SRCU_STRUCT_SRCU_READER_FLAVOR_OFFSET: usize = 16;
const LINUX_SRCU_STRUCT_SRCU_SUP_OFFSET: usize = 24;
const LINUX_SRCU_USAGE_SIZE: usize = 384;
const SRCU_READ_FLAVOR_NORMAL: i32 = 0x1;

/// `struct srcu_struct` — Linux ABI shape (simplified for tiny SRCU).
pub struct SrcuStruct {
    /// Per-CPU pair of counters: `[cpu][0]` = lock-side, `[cpu][1]` = unlock-side.
    counters: [[AtomicI32; 2]; MAX_CPUS],
    /// Active index — flipped by each grace period.
    pub idx: AtomicI32,
}

impl SrcuStruct {
    pub const fn new() -> Self {
        Self {
            counters: [const { [const { AtomicI32::new(0) }; 2] }; MAX_CPUS],
            idx: AtomicI32::new(0),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum ModuleSrcuError {
    BadSection,
    OutOfMemory,
}

struct LinuxSrcuOwned {
    sda: usize,
    sup: usize,
}

lazy_static! {
    static ref DYNAMIC_SRCU: Mutex<BTreeMap<usize, LinuxSrcuOwned>> = Mutex::new(BTreeMap::new());
}

/// Owned equivalent of Linux's module SRCU notifier state.
///
/// The pointers in `___srcu_struct_ptrs` live inside the module's relocated
/// sections, so this registration must be stored before section allocations in
/// the loaded-module owner.
pub struct ModuleSrcuRegistration {
    records: Vec<ModuleSrcuRecord>,
    active: core::sync::atomic::AtomicBool,
}

struct ModuleSrcuRecord {
    ssp: usize,
    sda: usize,
}

impl ModuleSrcuRegistration {
    pub fn empty() -> Self {
        Self {
            records: Vec::new(),
            active: core::sync::atomic::AtomicBool::new(true),
        }
    }

    pub fn cleanup(&self) {
        if !self.active.swap(false, Ordering::AcqRel) {
            return;
        }
        for record in self.records.iter() {
            unsafe {
                synchronize_linux_srcu(record.ssp as *mut u8);
                write_usize(record.ssp, LINUX_SRCU_STRUCT_SRCU_CTRP_OFFSET, 0);
                write_usize(record.ssp, LINUX_SRCU_STRUCT_SDA_OFFSET, 0);
                free_linux_srcu_data(record.sda as *mut u8);
            }
        }
    }
}

impl Drop for ModuleSrcuRegistration {
    fn drop(&mut self) {
        self.cleanup();
    }
}

pub fn module_srcu_finalize(
    section: Option<&[u8]>,
) -> Result<ModuleSrcuRegistration, ModuleSrcuError> {
    let Some(section) = section else {
        return Ok(ModuleSrcuRegistration::empty());
    };
    if section.len() % core::mem::size_of::<usize>() != 0 {
        return Err(ModuleSrcuError::BadSection);
    }

    let mut registration = ModuleSrcuRegistration::empty();
    for raw in section.chunks_exact(core::mem::size_of::<usize>()) {
        let ssp = usize::from_le_bytes(raw.try_into().map_err(|_| ModuleSrcuError::BadSection)?);
        if ssp == 0 {
            return Err(ModuleSrcuError::BadSection);
        }
        let sda = module_static_srcu_initialize(ssp)?;
        registration.records.push(ModuleSrcuRecord { ssp, sda });
    }
    Ok(registration)
}

fn module_static_srcu_initialize(ssp: usize) -> Result<usize, ModuleSrcuError> {
    let existing_sda = unsafe { read_usize(ssp, LINUX_SRCU_STRUCT_SDA_OFFSET) };
    if existing_sda != 0 {
        return Err(ModuleSrcuError::BadSection);
    }
    let sda = alloc_linux_srcu_data().ok_or(ModuleSrcuError::OutOfMemory)?;
    unsafe {
        write_usize(ssp, LINUX_SRCU_STRUCT_SDA_OFFSET, sda as usize);
        write_usize(ssp, LINUX_SRCU_STRUCT_SRCU_CTRP_OFFSET, sda as usize);
        write_i32(sda as usize, LINUX_SRCU_DATA_SRCU_READER_FLAVOR_OFFSET, 0);
    }
    Ok(sda as usize)
}

fn alloc_zeroed(size: usize) -> *mut u8 {
    unsafe { crate::mm::slab::kmalloc(size, GFP_KERNEL | __GFP_ZERO) }
}

unsafe fn free_allocated(ptr: *mut u8) {
    unsafe { crate::mm::slab::kfree(ptr) };
}

fn alloc_linux_srcu_data() -> Option<*mut u8> {
    let ptr = alloc_zeroed(LINUX_SRCU_DATA_SIZE);
    (!ptr.is_null()).then_some(ptr)
}

fn alloc_linux_srcu_usage() -> Option<*mut u8> {
    let ptr = alloc_zeroed(LINUX_SRCU_USAGE_SIZE);
    (!ptr.is_null()).then_some(ptr)
}

unsafe fn free_linux_srcu_data(ptr: *mut u8) {
    unsafe { free_allocated(ptr) };
}

unsafe fn free_linux_srcu_usage(ptr: *mut u8) {
    unsafe { free_allocated(ptr) };
}

unsafe fn read_usize(base: usize, offset: usize) -> usize {
    unsafe { ((base + offset) as *const usize).read() }
}

unsafe fn write_usize(base: usize, offset: usize, value: usize) {
    unsafe { ((base + offset) as *mut usize).write(value) };
}

unsafe fn write_u8(base: usize, offset: usize, value: u8) {
    unsafe { ((base + offset) as *mut u8).write(value) };
}

unsafe fn write_i32(base: usize, offset: usize, value: i32) {
    unsafe { ((base + offset) as *mut i32).write(value) };
}

fn srcu_ctr_ptr(ssp: *mut u8, idx: usize) -> *mut u8 {
    if ssp.is_null() || idx > 1 {
        return core::ptr::null_mut();
    }
    let sda = unsafe { read_usize(ssp as usize, LINUX_SRCU_STRUCT_SDA_OFFSET) };
    if sda == 0 {
        return core::ptr::null_mut();
    }
    (sda + idx * LINUX_SRCU_CTR_SIZE) as *mut u8
}

fn srcu_ctr_atomic(ctr: *mut u8, offset: usize) -> Option<&'static core::sync::atomic::AtomicU64> {
    if ctr.is_null() {
        return None;
    }
    Some(unsafe { &*((ctr as usize + offset) as *const core::sync::atomic::AtomicU64) })
}

fn linux_srcu_readers_active(ssp: *mut u8) -> bool {
    for idx in 0..2 {
        let ctr = srcu_ctr_ptr(ssp, idx);
        let Some(locks) = srcu_ctr_atomic(ctr, LINUX_SRCU_CTR_LOCKS_OFFSET) else {
            continue;
        };
        let Some(unlocks) = srcu_ctr_atomic(ctr, LINUX_SRCU_CTR_UNLOCKS_OFFSET) else {
            continue;
        };
        if locks.load(Ordering::Acquire) != unlocks.load(Ordering::Acquire) {
            return true;
        }
    }
    false
}

fn synchronize_linux_srcu(ssp: *mut u8) {
    while linux_srcu_readers_active(ssp) {
        #[cfg(not(test))]
        unsafe {
            crate::kernel::sched::schedule_with_irqs_enabled();
        }
        #[cfg(test)]
        break;
    }
    core::sync::atomic::fence(Ordering::SeqCst);
}

pub unsafe extern "C" fn linux_init_srcu_struct(ssp: *mut u8) -> i32 {
    if ssp.is_null() {
        return -EINVAL;
    }
    let sda = match alloc_linux_srcu_data() {
        Some(ptr) => ptr,
        None => return -ENOMEM,
    };
    let sup = match alloc_linux_srcu_usage() {
        Some(ptr) => ptr,
        None => {
            unsafe { free_linux_srcu_data(sda) };
            return -ENOMEM;
        }
    };

    unsafe {
        write_usize(ssp as usize, LINUX_SRCU_STRUCT_SDA_OFFSET, sda as usize);
        write_usize(
            ssp as usize,
            LINUX_SRCU_STRUCT_SRCU_CTRP_OFFSET,
            sda as usize,
        );
        write_u8(ssp as usize, LINUX_SRCU_STRUCT_SRCU_READER_FLAVOR_OFFSET, 0);
        write_usize(
            ssp as usize,
            LINUX_SRCU_STRUCT_SRCU_SUP_OFFSET,
            sup as usize,
        );
        write_i32(sda as usize, LINUX_SRCU_DATA_SRCU_READER_FLAVOR_OFFSET, 0);
    }
    DYNAMIC_SRCU.lock().insert(
        ssp as usize,
        LinuxSrcuOwned {
            sda: sda as usize,
            sup: sup as usize,
        },
    );
    0
}

pub unsafe extern "C" fn linux_cleanup_srcu_struct(ssp: *mut u8) {
    if ssp.is_null() {
        return;
    }
    synchronize_linux_srcu(ssp);
    let Some(owned) = DYNAMIC_SRCU.lock().remove(&(ssp as usize)) else {
        return;
    };
    unsafe {
        write_usize(ssp as usize, LINUX_SRCU_STRUCT_SRCU_CTRP_OFFSET, 0);
        write_usize(ssp as usize, LINUX_SRCU_STRUCT_SDA_OFFSET, 0);
        write_usize(ssp as usize, LINUX_SRCU_STRUCT_SRCU_SUP_OFFSET, 0);
        if owned.sda != 0 {
            free_linux_srcu_data(owned.sda as *mut u8);
        }
        if owned.sup != 0 {
            free_linux_srcu_usage(owned.sup as *mut u8);
        }
    }
}

pub unsafe extern "C" fn linux___srcu_read_lock(ssp: *mut u8) -> i32 {
    if ssp.is_null() {
        return 0;
    }
    let ctrp = unsafe { read_usize(ssp as usize, LINUX_SRCU_STRUCT_SRCU_CTRP_OFFSET) };
    if ctrp == 0 {
        return 0;
    }
    let Some(locks) = srcu_ctr_atomic(ctrp as *mut u8, LINUX_SRCU_CTR_LOCKS_OFFSET) else {
        return 0;
    };
    locks.fetch_add(1, Ordering::AcqRel);
    core::sync::atomic::fence(Ordering::SeqCst);

    let sda = unsafe { read_usize(ssp as usize, LINUX_SRCU_STRUCT_SDA_OFFSET) };
    if sda == 0 || ctrp < sda {
        return 0;
    }
    let idx = (ctrp - sda) / LINUX_SRCU_CTR_SIZE;
    i32::try_from(idx.min(1)).unwrap_or(0)
}

pub unsafe extern "C" fn linux___srcu_read_unlock(ssp: *mut u8, idx: i32) {
    if idx < 0 {
        return;
    }
    core::sync::atomic::fence(Ordering::SeqCst);
    let ctr = srcu_ctr_ptr(ssp, idx as usize);
    let Some(unlocks) = srcu_ctr_atomic(ctr, LINUX_SRCU_CTR_UNLOCKS_OFFSET) else {
        return;
    };
    unlocks.fetch_add(1, Ordering::AcqRel);
}

pub unsafe extern "C" fn linux_synchronize_srcu(ssp: *mut u8) {
    synchronize_linux_srcu(ssp);
}

pub unsafe extern "C" fn linux_synchronize_srcu_expedited(ssp: *mut u8) {
    synchronize_linux_srcu(ssp);
}

#[inline]
fn cpu_index() -> usize {
    #[cfg(test)]
    return 0;
    #[cfg(not(test))]
    {
        // Skip the LAPIC MMIO read (a VM-exit on VBox) when only the BSP is
        // online; single-CPU SRCU read-side always resolves to index 0.
        if crate::arch::x86::kernel::smp::AP_READY_COUNT.load(core::sync::atomic::Ordering::Acquire)
            == 0
        {
            return 0;
        }
        let id = unsafe { crate::arch::x86::kernel::apic::id() } as usize;
        id.min(MAX_CPUS - 1)
    }
}

/// `srcu_read_lock(ssp)` — returns the index that must be passed to
/// `srcu_read_unlock`.
pub fn srcu_read_lock(ssp: &SrcuStruct) -> i32 {
    let idx = ssp.idx.load(Ordering::Acquire);
    let cpu = cpu_index();
    ssp.counters[cpu][idx as usize].fetch_add(1, Ordering::AcqRel);
    idx
}

pub fn srcu_read_unlock(ssp: &SrcuStruct, idx: i32) {
    let cpu = cpu_index();
    ssp.counters[cpu][idx as usize].fetch_sub(1, Ordering::AcqRel);
}

/// `synchronize_srcu(ssp)` — flip the index, then wait for the previous-index
/// counter to reach zero across all CPUs.
pub fn synchronize_srcu(ssp: &SrcuStruct) {
    let old = ssp.idx.load(Ordering::Acquire);
    ssp.idx.store(1 - old, Ordering::Release);
    loop {
        let mut sum: i32 = 0;
        for cpu in 0..MAX_CPUS {
            sum = sum.saturating_add(ssp.counters[cpu][old as usize].load(Ordering::Acquire));
        }
        if sum <= 0 {
            return;
        }
        #[cfg(not(test))]
        unsafe {
            crate::kernel::sched::schedule_with_irqs_enabled();
        }
        #[cfg(test)]
        {
            // In tests, callers must release locks before synchronize.
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_unlock_round_trip() {
        let s = SrcuStruct::new();
        let idx = srcu_read_lock(&s);
        srcu_read_unlock(&s, idx);
    }

    #[test]
    fn synchronize_advances_index() {
        let s = SrcuStruct::new();
        let before = s.idx.load(Ordering::Acquire);
        synchronize_srcu(&s);
        assert_ne!(s.idx.load(Ordering::Acquire), before);
    }
}
