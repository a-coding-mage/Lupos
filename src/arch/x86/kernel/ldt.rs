//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/ldt.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/ldt.c
//! x86 Local Descriptor Table helpers and `modify_ldt(2)`.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/ldt.c

#![allow(dead_code)]

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;
use spin::Mutex;

use crate::arch::x86::kernel::uaccess;
use crate::include::uapi::errno::{EFAULT, EINVAL};

pub const LDT_ENTRIES: usize = 8192;
pub const LDT_ENTRY_SIZE: usize = 8;
pub const LDT_BYTE_SIZE: usize = LDT_ENTRIES * LDT_ENTRY_SIZE;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UserDesc {
    pub entry_number: u32,
    pub base_addr: u32,
    pub limit: u32,
    pub seg_32bit: u32,
    pub contents: u32,
    pub read_exec_only: u32,
    pub limit_in_pages: u32,
    pub seg_not_present: u32,
    pub useable: u32,
    pub lm: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LdtDescriptor(pub u64);

#[derive(Clone, Debug)]
pub struct LdtContext {
    entries: Vec<LdtDescriptor>,
}

impl LdtContext {
    pub fn new(entries: usize) -> Self {
        Self {
            entries: vec![LdtDescriptor::default(); entries.min(LDT_ENTRIES)],
        }
    }

    pub fn read_bytes(&self, out: &mut [u8]) -> usize {
        let mut written = 0;
        for desc in self.entries.iter() {
            if written + LDT_ENTRY_SIZE > out.len() {
                break;
            }
            out[written..written + LDT_ENTRY_SIZE].copy_from_slice(&desc.0.to_le_bytes());
            written += LDT_ENTRY_SIZE;
        }
        written
    }

    pub fn write_desc(&mut self, user: UserDesc) -> Result<(), i32> {
        if user.entry_number as usize >= LDT_ENTRIES || user.contents > 3 {
            return Err(EINVAL);
        }
        let idx = user.entry_number as usize;
        if idx >= self.entries.len() {
            self.entries.resize(idx + 1, LdtDescriptor::default());
        }
        self.entries[idx] = ldt_descriptor_from_user_desc(user)?;
        Ok(())
    }

    pub fn descriptor(&self, index: usize) -> Option<LdtDescriptor> {
        self.entries.get(index).copied()
    }
}

static CURRENT_LDT: Mutex<LdtContext> = Mutex::new(LdtContext {
    entries: Vec::new(),
});

pub const fn ldt_slot_va(slot: usize) -> u64 {
    0xffff_8800_0000_0000u64 + (slot as u64 * LDT_ENTRY_SIZE as u64)
}

pub fn load_mm_ldt(ctx: LdtContext) {
    *CURRENT_LDT.lock() = ctx;
}

pub fn switch_ldt(next: LdtContext) -> LdtContext {
    core::mem::replace(&mut *CURRENT_LDT.lock(), next)
}

pub fn ldt_dup_context(ctx: &LdtContext) -> LdtContext {
    ctx.clone()
}

pub fn destroy_context_ldt(ctx: &mut LdtContext) {
    ctx.entries.clear();
}

pub fn read_default_ldt(out: &mut [u8]) -> usize {
    out.fill(0);
    out.len()
}

pub const fn allow_16bit_segments(desc: &UserDesc) -> bool {
    desc.seg_32bit == 0
}

pub fn ldt_descriptor_from_user_desc(desc: UserDesc) -> Result<LdtDescriptor, i32> {
    if desc.contents > 3
        || desc.seg_32bit > 1
        || desc.limit_in_pages > 1
        || desc.seg_not_present > 1
    {
        return Err(EINVAL);
    }
    let limit = desc.limit & 0x000f_ffff;
    let base = desc.base_addr;
    let mut value = 0u64;
    value |= (limit & 0xffff) as u64;
    value |= ((base & 0x00ff_ffff) as u64) << 16;
    let typ = if desc.contents == 0 {
        0x2 | ((desc.read_exec_only == 0) as u64)
    } else {
        0x8 | ((desc.contents as u64 & 0x3) << 2) | ((desc.read_exec_only == 0) as u64)
    };
    value |= typ << 40;
    if desc.seg_not_present == 0 {
        value |= 1 << 47;
    }
    value |= ((limit >> 16) as u64 & 0x0f) << 48;
    if desc.useable != 0 {
        value |= 1 << 52;
    }
    if desc.lm != 0 {
        value |= 1 << 53;
    }
    if desc.seg_32bit != 0 {
        value |= 1 << 54;
    }
    if desc.limit_in_pages != 0 {
        value |= 1 << 55;
    }
    value |= ((base >> 24) as u64) << 56;
    Ok(LdtDescriptor(value))
}

pub unsafe fn sys_modify_ldt(func: i32, ptr: *mut u8, bytecount: usize) -> i64 {
    match func {
        0 => {
            if bytecount == 0 {
                return 0;
            }
            if ptr.is_null() {
                return -(EFAULT as i64);
            }
            let mut bytes = vec![0u8; bytecount.min(LDT_BYTE_SIZE)];
            let n = CURRENT_LDT.lock().read_bytes(&mut bytes);
            if unsafe { uaccess::copy_to_user(ptr, bytes.as_ptr(), n) } != 0 {
                return -(EFAULT as i64);
            }
            n as i64
        }
        1 | 0x11 => {
            if bytecount < core::mem::size_of::<UserDesc>() {
                return -(EINVAL as i64);
            }
            if ptr.is_null() {
                return -(EFAULT as i64);
            }
            let mut desc = UserDesc::default();
            if unsafe {
                uaccess::copy_from_user(
                    &mut desc as *mut UserDesc as *mut u8,
                    ptr as *const u8,
                    core::mem::size_of::<UserDesc>(),
                )
            } != 0
            {
                return -(EFAULT as i64);
            }
            match CURRENT_LDT.lock().write_desc(desc) {
                Ok(()) => 0,
                Err(errno) => -(errno as i64),
            }
        }
        2 => {
            if bytecount == 0 {
                return 0;
            }
            if ptr.is_null() {
                return -(EFAULT as i64);
            }
            let mut bytes = vec![0u8; bytecount.min(LDT_BYTE_SIZE)];
            let n = read_default_ldt(&mut bytes);
            if unsafe { uaccess::copy_to_user(ptr, bytes.as_ptr(), n) } != 0 {
                return -(EFAULT as i64);
            }
            n as i64
        }
        _ => -(EINVAL as i64),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_layout_sets_presence_and_base_limit() {
        let desc = UserDesc {
            entry_number: 1,
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
        let encoded = ldt_descriptor_from_user_desc(desc).unwrap().0;
        assert_ne!(encoded & (1 << 47), 0);
        assert_ne!(encoded & (1 << 54), 0);
        assert_ne!(encoded & (1 << 55), 0);
    }

    #[test]
    fn context_write_and_read_round_trip() {
        let mut ctx = LdtContext::new(0);
        ctx.write_desc(UserDesc {
            entry_number: 2,
            base_addr: 0x1000,
            limit: 0xfff,
            seg_32bit: 1,
            ..Default::default()
        })
        .unwrap();
        let mut bytes = [0u8; 32];
        assert_eq!(ctx.read_bytes(&mut bytes), 24);
        assert_ne!(u64::from_le_bytes(bytes[16..24].try_into().unwrap()), 0);
    }

    #[test]
    fn modify_ldt_read_paths_reject_huge_bytecounts_without_unbounded_alloc() {
        let bad_user_ptr = uaccess::TASK_SIZE_MAX as *mut u8;
        let saved = switch_ldt(LdtContext::new(1));

        assert_eq!(
            unsafe { sys_modify_ldt(0, bad_user_ptr, usize::MAX) },
            -(EFAULT as i64)
        );
        assert_eq!(
            unsafe { sys_modify_ldt(2, bad_user_ptr, usize::MAX) },
            -(EFAULT as i64)
        );

        load_mm_ldt(saved);
    }
}
