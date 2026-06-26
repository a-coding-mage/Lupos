//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/tls.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/tls.c
//! x86 TLS descriptor syscalls.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/tls.c

#![allow(dead_code)]

use crate::arch::x86::kernel::ldt::{UserDesc, ldt_descriptor_from_user_desc};
use crate::arch::x86::kernel::uaccess;
use crate::include::uapi::errno::{EFAULT, EINVAL, ESRCH};
use crate::kernel::sched;
use crate::kernel::task::TaskStruct;
use crate::kernel::thread::{DescStruct, ThreadStruct};

pub const GDT_ENTRY_TLS_MIN: i32 = 12;
pub const GDT_ENTRY_TLS_ENTRIES: usize = 3;
pub const GDT_ENTRY_TLS_MAX: i32 = GDT_ENTRY_TLS_MIN + GDT_ENTRY_TLS_ENTRIES as i32 - 1;

pub const fn ldt_zero(info: &UserDesc) -> bool {
    info.entry_number == 0
        && info.base_addr == 0
        && info.limit == 0
        && info.seg_32bit == 0
        && info.contents == 0
        && info.read_exec_only == 0
        && info.limit_in_pages == 0
        && info.seg_not_present == 0
        && info.useable == 0
        && info.lm == 0
}

pub const fn ldt_empty(info: &UserDesc) -> bool {
    info.read_exec_only == 1 && info.seg_not_present == 1 && info.base_addr == 0 && info.limit == 0
}

pub const fn tls_desc_okay(info: &UserDesc) -> bool {
    if ldt_empty(info) || ldt_zero(info) {
        return true;
    }
    if info.seg_32bit == 0 || info.contents > 1 || info.seg_not_present != 0 {
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
        return UserDesc {
            entry_number: idx as u32,
            read_exec_only: 1,
            seg_not_present: 1,
            ..UserDesc::default()
        };
    }
    let value = desc.0;
    let limit_low = value & 0xffff;
    let limit_high = (value >> 48) & 0x0f;
    let base_low = (value >> 16) & 0x00ff_ffff;
    let base_high = (value >> 56) & 0xff;
    let typ = (value >> 40) & 0x0f;
    UserDesc {
        entry_number: idx as u32,
        base_addr: (base_low | (base_high << 24)) as u32,
        limit: (limit_low | (limit_high << 16)) as u32,
        seg_32bit: ((value >> 54) & 1) as u32,
        contents: ((typ >> 2) & 0x3) as u32,
        read_exec_only: ((typ & 0x2) == 0) as u32,
        limit_in_pages: ((value >> 55) & 1) as u32,
        seg_not_present: ((value & (1 << 47)) == 0) as u32,
        useable: ((value >> 52) & 1) as u32,
        lm: ((value >> 53) & 1) as u32,
    }
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
                info.entry_number = free as u32;
                let copied = unsafe {
                    uaccess::copy_to_user(
                        u_info as *mut u8,
                        &info as *const UserDesc as *const u8,
                        core::mem::size_of::<UserDesc>(),
                    )
                };
                if copied != 0 {
                    return -(EFAULT as i64);
                }
            }
            Err(err) => return -(err as i64),
        }
    }
    match set_tls_desc(unsafe { &mut (*task).thread }, idx, &info) {
        Ok(()) => 0,
        Err(err) => -(err as i64),
    }
}

pub fn do_get_thread_area(task: *mut TaskStruct, mut idx: i32, u_info: *mut UserDesc) -> i64 {
    if task.is_null() || u_info.is_null() {
        return -(EFAULT as i64);
    }
    if idx == -1 {
        let mut tmp = UserDesc::default();
        let copied = unsafe {
            uaccess::copy_from_user(
                &mut tmp as *mut UserDesc as *mut u8,
                u_info as *const u8,
                core::mem::size_of::<UserDesc>(),
            )
        };
        if copied != 0 {
            return -(EFAULT as i64);
        }
        idx = tmp.entry_number as i32;
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
        assert!(!tls_desc_okay(&UserDesc {
            seg_32bit: 0,
            base_addr: 1,
            ..UserDesc::default()
        }));
        assert!(!tls_desc_okay(&UserDesc {
            seg_32bit: 1,
            seg_not_present: 1,
            ..UserDesc::default()
        }));
    }

    #[test]
    fn set_and_get_thread_area_round_trip_descriptor() {
        let mut task = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let mut desc = UserDesc {
            entry_number: u32::MAX,
            base_addr: 0x1234_5000,
            limit: 0xffff,
            seg_32bit: 1,
            contents: 0,
            read_exec_only: 0,
            limit_in_pages: 1,
            seg_not_present: 0,
            useable: 1,
            lm: 0,
        };
        assert_eq!(
            do_set_thread_area(&mut *task, -1, &mut desc as *mut UserDesc, true),
            0
        );
        assert_eq!(desc.entry_number, GDT_ENTRY_TLS_MIN as u32);
        let mut out = UserDesc {
            entry_number: desc.entry_number,
            ..UserDesc::default()
        };
        assert_eq!(
            do_get_thread_area(&mut *task, -1, &mut out as *mut UserDesc),
            0
        );
        assert_eq!(out.base_addr, 0x1234_5000);
    }
}
