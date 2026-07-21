//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/kernel/tls.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/tls.c
//! x86 TLS descriptor syscalls.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/tls.c
//!
//! The native x86-64 syscall path, GDT refresh, and current-task selector
//! invalidation are live. The separate IA32 `int $0x80` entry plumbing remains
//! incomplete elsewhere in the architecture port.

#![allow(dead_code)]

use crate::arch::x86::kernel::ldt::{UserDesc, ldt_descriptor_from_user_desc};
use crate::arch::x86::kernel::uaccess;
use crate::include::uapi::errno::{EFAULT, EINVAL, ESRCH};
use crate::kernel::sched;
use crate::kernel::task::TaskStruct;
use crate::kernel::thread::{DescStruct, ThreadStruct};

pub const GDT_ENTRY_TLS_MIN: i32 = crate::arch::x86::kernel::gdt::GDT_ENTRY_TLS_MIN as i32;
pub const GDT_ENTRY_TLS_ENTRIES: usize = crate::arch::x86::kernel::gdt::GDT_ENTRY_TLS_ENTRIES;
pub const GDT_ENTRY_TLS_MAX: i32 = crate::arch::x86::kernel::gdt::GDT_ENTRY_TLS_MAX as i32;

pub const fn ldt_zero(info: &UserDesc) -> bool {
    info.base_addr == 0
        && info.limit == 0
        && info.seg_32bit() == 0
        && info.contents() == 0
        && info.read_exec_only() == 0
        && info.limit_in_pages() == 0
        && info.seg_not_present() == 0
        && info.useable() == 0
}

pub const fn ldt_empty(info: &UserDesc) -> bool {
    info.base_addr == 0
        && info.limit == 0
        && info.contents() == 0
        && info.read_exec_only() == 1
        && info.seg_32bit() == 0
        && info.limit_in_pages() == 0
        && info.seg_not_present() == 1
        && info.useable() == 0
}

pub const fn tls_desc_okay(info: &UserDesc) -> bool {
    if ldt_empty(info) || ldt_zero(info) {
        return true;
    }
    if info.seg_32bit() == 0 || info.contents() > 1 || info.seg_not_present() != 0 {
        return false;
    }
    true
}

pub const fn desc_empty(desc: DescStruct) -> bool {
    desc.0 == 0
}

pub fn get_free_idx(thread: &ThreadStruct) -> Result<i32, i32> {
    for (idx, desc) in thread.tls_array.iter().enumerate() {
        if desc_empty(*desc) {
            return Ok(GDT_ENTRY_TLS_MIN + idx as i32);
        }
    }
    Err(ESRCH)
}

pub fn fill_user_desc(idx: i32, desc: DescStruct) -> UserDesc {
    if desc.0 == 0 {
        return UserDesc::new(idx as u32, 0, 0, 0, 0, 1, 0, 1, 0, 0);
    }
    let value = desc.0;
    let limit_low = value & 0xffff;
    let limit_high = (value >> 48) & 0x0f;
    let base_low = (value >> 16) & 0x00ff_ffff;
    let base_high = (value >> 56) & 0xff;
    let typ = (value >> 40) & 0x0f;
    UserDesc::new(
        idx as u32,
        (base_low | (base_high << 24)) as u32,
        (limit_low | (limit_high << 16)) as u32,
        ((value >> 54) & 1) as u32,
        ((typ >> 2) & 0x3) as u32,
        ((typ & 0x2) == 0) as u32,
        ((value >> 55) & 1) as u32,
        ((value & (1 << 47)) == 0) as u32,
        ((value >> 52) & 1) as u32,
        ((value >> 53) & 1) as u32,
    )
}

pub fn set_tls_desc(thread: &mut ThreadStruct, idx: i32, info: &UserDesc) -> Result<(), i32> {
    if !(GDT_ENTRY_TLS_MIN..=GDT_ENTRY_TLS_MAX).contains(&idx) || !tls_desc_okay(info) {
        return Err(EINVAL);
    }
    let slot = (idx - GDT_ENTRY_TLS_MIN) as usize;
    thread.tls_array[slot] = if ldt_empty(info) || ldt_zero(info) {
        DescStruct(0)
    } else {
        DescStruct(ldt_descriptor_from_user_desc(*info)?.0)
    };
    Ok(())
}

#[inline]
const fn refreshed_selector(live: u16, modified: u16, descriptor_empty: bool) -> Option<u16> {
    if live != modified {
        None
    } else if descriptor_empty {
        Some(0)
    } else {
        Some(modified)
    }
}

#[cfg(not(test))]
fn set_task_tls_desc(task: *mut TaskStruct, idx: i32, info: &UserDesc) -> Result<(), i32> {
    // Linux set_tls_desc() pins the task to its CPU while changing
    // current->thread.tls_array and that CPU's live GDT copy.
    crate::kernel::locking::preempt::preempt_disable();
    let result = unsafe { set_tls_desc(&mut (*task).thread, idx, info) };
    if result.is_ok() && task == unsafe { sched::get_current() } {
        let cpu = crate::arch::x86::kernel::setup_percpu::current_cpu_number();
        unsafe {
            crate::arch::x86::kernel::gdt::load_tls(&(*task).thread, cpu);
        }
    }
    crate::kernel::locking::preempt::preempt_enable();
    result
}

#[cfg(test)]
fn set_task_tls_desc(task: *mut TaskStruct, idx: i32, info: &UserDesc) -> Result<(), i32> {
    unsafe { set_tls_desc(&mut (*task).thread, idx, info) }
}

#[cfg(not(test))]
unsafe fn refresh_current_tls_selector(task: *mut TaskStruct, idx: i32) {
    let modified = ((idx as u16) << 3) | 3;
    let slot = (idx - GDT_ENTRY_TLS_MIN) as usize;
    let empty = unsafe { desc_empty((*task).thread.tls_array[slot]) };

    if let Some(selector) = refreshed_selector(
        unsafe { crate::arch::x86::kernel::gdt::read_ds() },
        modified,
        empty,
    ) {
        unsafe {
            crate::arch::x86::kernel::gdt::load_ds(selector);
        }
    }
    if let Some(selector) = refreshed_selector(
        unsafe { crate::arch::x86::kernel::gdt::read_es() },
        modified,
        empty,
    ) {
        unsafe {
            crate::arch::x86::kernel::gdt::load_es(selector);
        }
    }
    if let Some(selector) = refreshed_selector(
        unsafe { crate::arch::x86::kernel::gdt::read_fs() },
        modified,
        empty,
    ) {
        unsafe {
            crate::arch::x86::kernel::gdt::load_fs(selector);
            if empty {
                // Linux's EX_TYPE_CLEAR_FS fixup guarantees that invalidating
                // an active TLS descriptor also clears the hidden FS base.
                crate::arch::x86::kernel::msr::write(crate::arch::x86::kernel::msr::MSR_FS_BASE, 0);
            }
        }
    }
    if let Some(selector) = refreshed_selector(
        unsafe { crate::arch::x86::kernel::gdt::read_gs() },
        modified,
        empty,
    ) {
        unsafe {
            crate::arch::x86::kernel::gdt::load_gs_index(selector);
            if empty {
                crate::arch::x86::kernel::msr::write(
                    crate::arch::x86::kernel::msr::MSR_KERNEL_GS_BASE,
                    0,
                );
            }
        }
    }
}

#[cfg(test)]
unsafe fn refresh_current_tls_selector(_task: *mut TaskStruct, _idx: i32) {}

pub fn do_set_thread_area(
    task: *mut TaskStruct,
    mut idx: i32,
    u_info: *mut UserDesc,
    can_allocate: bool,
) -> i64 {
    if task.is_null() || u_info.is_null() {
        return -(EFAULT as i64);
    }
    let mut info = UserDesc::default();
    let copied = unsafe {
        uaccess::copy_from_user(
            &mut info as *mut UserDesc as *mut u8,
            u_info as *const u8,
            core::mem::size_of::<UserDesc>(),
        )
    };
    if copied != 0 {
        return -(EFAULT as i64);
    }
    if !tls_desc_okay(&info) {
        return -(EINVAL as i64);
    }
    if idx == -1 {
        idx = info.entry_number as i32;
    }
    if idx == -1 && can_allocate {
        match get_free_idx(unsafe { &(*task).thread }) {
            Ok(free) => {
                idx = free;
                let allocated = free as u32;
                // Linux put_user() updates only entry_number, not the entire
                // descriptor supplied by userspace.
                let copied = unsafe {
                    uaccess::copy_to_user(
                        u_info as *mut u8,
                        &allocated as *const u32 as *const u8,
                        core::mem::size_of::<u32>(),
                    )
                };
                if copied != 0 {
                    return -(EFAULT as i64);
                }
            }
            Err(err) => return -(err as i64),
        }
    }
    match set_task_tls_desc(task, idx, &info) {
        Ok(()) => {
            let modified = ((idx as u16) << 3) | 3;
            if task == unsafe { sched::get_current() } {
                unsafe {
                    refresh_current_tls_selector(task, idx);
                }
            } else {
                unsafe {
                    if (*task).thread.fsindex == modified {
                        (*task).thread.fsbase = info.base_addr as u64;
                    }
                    if (*task).thread.gsindex == modified {
                        (*task).thread.gsbase = info.base_addr as u64;
                    }
                }
            }
            0
        }
        Err(err) => -(err as i64),
    }
}

pub fn do_get_thread_area(task: *mut TaskStruct, mut idx: i32, u_info: *mut UserDesc) -> i64 {
    if task.is_null() || u_info.is_null() {
        return -(EFAULT as i64);
    }
    if idx == -1 {
        let mut entry_number = 0u32;
        let copied = unsafe {
            uaccess::copy_from_user(
                &mut entry_number as *mut u32 as *mut u8,
                u_info as *const u8,
                core::mem::size_of::<u32>(),
            )
        };
        if copied != 0 {
            return -(EFAULT as i64);
        }
        idx = entry_number as i32;
    }
    if !(GDT_ENTRY_TLS_MIN..=GDT_ENTRY_TLS_MAX).contains(&idx) {
        return -(EINVAL as i64);
    }
    let slot = (idx - GDT_ENTRY_TLS_MIN) as usize;
    let info = fill_user_desc(idx, unsafe { (*task).thread.tls_array[slot] });
    let copied = unsafe {
        uaccess::copy_to_user(
            u_info as *mut u8,
            &info as *const UserDesc as *const u8,
            core::mem::size_of::<UserDesc>(),
        )
    };
    if copied == 0 { 0 } else { -(EFAULT as i64) }
}

pub unsafe fn sys_set_thread_area(u_info: *mut UserDesc) -> i64 {
    do_set_thread_area(unsafe { sched::get_current() }, -1, u_info, true)
}

pub unsafe fn sys_get_thread_area(u_info: *mut UserDesc) -> i64 {
    do_get_thread_area(unsafe { sched::get_current() }, -1, u_info)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::task::TaskStruct;
    use alloc::boxed::Box;

    #[test]
    fn tls_rejects_16bit_or_nonpresent_segments() {
        assert!(tls_desc_okay(&UserDesc::default()));
        assert!(!tls_desc_okay(
            &UserDesc::new(0, 1, 0, 0, 0, 0, 0, 0, 0, 0,)
        ));
        assert!(!tls_desc_okay(
            &UserDesc::new(0, 1, 0, 1, 0, 0, 0, 1, 0, 0,)
        ));
    }

    #[test]
    fn set_and_get_thread_area_round_trip_descriptor() {
        let mut task = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let mut desc = UserDesc::new(u32::MAX, 0x1234_5000, 0xffff, 1, 0, 0, 1, 0, 1, 0);
        assert_eq!(
            do_set_thread_area(&mut *task, -1, &mut desc as *mut UserDesc, true),
            0
        );
        assert_eq!(desc.entry_number, GDT_ENTRY_TLS_MIN as u32);
        let mut out = UserDesc::new(desc.entry_number, 0, 0, 0, 0, 0, 0, 0, 0, 0);
        assert_eq!(
            do_get_thread_area(&mut *task, -1, &mut out as *mut UserDesc),
            0
        );
        assert_eq!(out.base_addr, 0x1234_5000);
    }

    #[test]
    fn clearing_a_modified_live_selector_selects_null() {
        // Origin: vendor/linux/tools/testing/selftests/x86/ldt_gdt.c
        // `test_gdt_invalidation`.
        let modified = ((GDT_ENTRY_TLS_MIN as u16) << 3) | 3;
        assert_eq!(
            refreshed_selector(modified, modified, true),
            Some(0),
            "an invalidated live TLS selector must become null"
        );
        assert_eq!(
            refreshed_selector(modified, modified, false),
            Some(modified),
            "a changed live descriptor must be reloaded"
        );
        assert_eq!(refreshed_selector(0, modified, true), None);
    }

    #[test]
    fn zero_and_empty_descriptors_ignore_entry_number_and_lm() {
        // Origin: vendor/linux/arch/x86/include/asm/desc.h LDT_zero/LDT_empty.
        let zero = UserDesc::new(u32::MAX, 0, 0, 0, 0, 0, 0, 0, 0, 1);
        let empty = UserDesc::new(u32::MAX, 0, 0, 0, 0, 1, 0, 1, 0, 1);
        assert!(ldt_zero(&zero));
        assert!(ldt_empty(&empty));
    }
}
