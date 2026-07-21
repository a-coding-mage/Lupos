//! linux-parity: partial
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
    flags: u32,
}

impl UserDesc {
    const SEG_32BIT: u32 = 1 << 0;
    const CONTENTS_SHIFT: u32 = 1;
    const CONTENTS_MASK: u32 = 0x3 << Self::CONTENTS_SHIFT;
    const READ_EXEC_ONLY: u32 = 1 << 3;
    const LIMIT_IN_PAGES: u32 = 1 << 4;
    const SEG_NOT_PRESENT: u32 = 1 << 5;
    const USEABLE: u32 = 1 << 6;
    const LM: u32 = 1 << 7;

    /// Construct the exact 16-byte x86 `struct user_desc` userspace ABI.
    ///
    /// Linux represents the final eight properties as bitfields in one
    /// `unsigned int`. Keeping the packed word explicit avoids relying on a
    /// Rust bitfield extension while preserving the C layout byte-for-byte.
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        entry_number: u32,
        base_addr: u32,
        limit: u32,
        seg_32bit: u32,
        contents: u32,
        read_exec_only: u32,
        limit_in_pages: u32,
        seg_not_present: u32,
        useable: u32,
        lm: u32,
    ) -> Self {
        Self {
            entry_number,
            base_addr,
            limit,
            flags: ((seg_32bit & 1) * Self::SEG_32BIT)
                | ((contents & 0x3) << Self::CONTENTS_SHIFT)
                | ((read_exec_only & 1) * Self::READ_EXEC_ONLY)
                | ((limit_in_pages & 1) * Self::LIMIT_IN_PAGES)
                | ((seg_not_present & 1) * Self::SEG_NOT_PRESENT)
                | ((useable & 1) * Self::USEABLE)
                | ((lm & 1) * Self::LM),
        }
    }

    pub const fn seg_32bit(self) -> u32 {
        (self.flags & Self::SEG_32BIT != 0) as u32
    }

    pub const fn contents(self) -> u32 {
        (self.flags & Self::CONTENTS_MASK) >> Self::CONTENTS_SHIFT
    }

    pub const fn read_exec_only(self) -> u32 {
        (self.flags & Self::READ_EXEC_ONLY != 0) as u32
    }

    pub const fn limit_in_pages(self) -> u32 {
        (self.flags & Self::LIMIT_IN_PAGES != 0) as u32
    }

    pub const fn seg_not_present(self) -> u32 {
        (self.flags & Self::SEG_NOT_PRESENT != 0) as u32
    }

    pub const fn useable(self) -> u32 {
        (self.flags & Self::USEABLE != 0) as u32
    }

    pub const fn lm(self) -> u32 {
        (self.flags & Self::LM != 0) as u32
    }
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
        if user.entry_number as usize >= LDT_ENTRIES {
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
    desc.seg_32bit() == 0
}

pub fn ldt_descriptor_from_user_desc(desc: UserDesc) -> Result<LdtDescriptor, i32> {
    let limit = desc.limit & 0x000f_ffff;
    let base = desc.base_addr;
    let mut value = 0u64;
    value |= (limit & 0xffff) as u64;
    value |= ((base & 0x00ff_ffff) as u64) << 16;
    let typ = (((desc.read_exec_only() ^ 1) << 1) | (desc.contents() << 2) | 1) as u64;
    value |= typ << 40;
    value |= 1 << 44;
    value |= 3 << 45;
    if desc.seg_not_present() == 0 {
        value |= 1 << 47;
    }
    value |= ((limit >> 16) as u64 & 0x0f) << 48;
    if desc.useable() != 0 {
        value |= 1 << 52;
    }
    // Linux fill_ldt() deliberately forces the long-mode bit to zero.
    if desc.seg_32bit() != 0 {
        value |= 1 << 54;
    }
    if desc.limit_in_pages() != 0 {
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
        let desc = UserDesc::new(1, 0x1234_5000, 0xffff, 1, 0, 0, 1, 0, 1, 0);
        let encoded = ldt_descriptor_from_user_desc(desc).unwrap().0;
        assert_ne!(encoded & (1 << 47), 0);
        assert_ne!(encoded & (1 << 44), 0);
        assert_eq!((encoded >> 45) & 3, 3);
        assert_eq!((encoded >> 40) & 0xf, 3);
        assert_ne!(encoded & (1 << 54), 0);
        assert_ne!(encoded & (1 << 55), 0);
    }

    #[test]
    fn user_desc_matches_linux_x86_64_uapi_layout() {
        // Origin: vendor/linux/arch/x86/include/uapi/asm/ldt.h
        assert_eq!(core::mem::size_of::<UserDesc>(), 16);
        assert_eq!(core::mem::offset_of!(UserDesc, entry_number), 0);
        assert_eq!(core::mem::offset_of!(UserDesc, base_addr), 4);
        assert_eq!(core::mem::offset_of!(UserDesc, limit), 8);
        assert_eq!(core::mem::offset_of!(UserDesc, flags), 12);

        let desc = UserDesc::new(12, 1, 2, 1, 2, 1, 1, 1, 1, 1);
        assert_eq!(desc.flags, 0xfd);
        assert_eq!(desc.seg_32bit(), 1);
        assert_eq!(desc.contents(), 2);
        assert_eq!(desc.read_exec_only(), 1);
        assert_eq!(desc.limit_in_pages(), 1);
        assert_eq!(desc.seg_not_present(), 1);
        assert_eq!(desc.useable(), 1);
        assert_eq!(desc.lm(), 1);
    }

    #[test]
    fn context_write_and_read_round_trip() {
        let mut ctx = LdtContext::new(0);
        ctx.write_desc(UserDesc::new(2, 0x1000, 0xfff, 1, 0, 0, 0, 0, 0, 0))
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
