// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: arch/x86/include/uapi/asm/ldt.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: x86_64
//! rewrite-task: S000779

/// Maximum number of LDT entries supported.
pub const LDT_ENTRIES: i32 = 8192;

/// The size, in bytes, of each LDT entry.
pub const LDT_ENTRY_SIZE: i32 = 8;

/// Definitions used with the `modify_ldt` system call.
///
/// This has the x86_64 C ABI of `struct user_desc`: three `unsigned int`
/// members followed by the complete 32-bit allocation unit that holds the C
/// bit-fields.  Keeping that allocation unit intact also preserves the
/// padding bits supplied by a 32-bit caller.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct user_desc {
    pub entry_number: u32,
    pub base_addr: u32,
    pub limit: u32,
    pub bits: u32,
}

impl user_desc {
    pub const SEG_32BIT_SHIFT: u32 = 0;
    pub const CONTENTS_SHIFT: u32 = 1;
    pub const READ_EXEC_ONLY_SHIFT: u32 = 3;
    pub const LIMIT_IN_PAGES_SHIFT: u32 = 4;
    pub const SEG_NOT_PRESENT_SHIFT: u32 = 5;
    pub const USEABLE_SHIFT: u32 = 6;
    pub const LM_SHIFT: u32 = 7;

    pub const SEG_32BIT_MASK: u32 = 0x0000_0001;
    pub const CONTENTS_MASK: u32 = 0x0000_0006;
    pub const READ_EXEC_ONLY_MASK: u32 = 0x0000_0008;
    pub const LIMIT_IN_PAGES_MASK: u32 = 0x0000_0010;
    pub const SEG_NOT_PRESENT_MASK: u32 = 0x0000_0020;
    pub const USEABLE_MASK: u32 = 0x0000_0040;
    pub const LM_MASK: u32 = 0x0000_0080;

    #[inline]
    pub const fn seg_32bit(&self) -> u32 {
        (self.bits & Self::SEG_32BIT_MASK) >> Self::SEG_32BIT_SHIFT
    }

    #[inline]
    pub const fn contents(&self) -> u32 {
        (self.bits & Self::CONTENTS_MASK) >> Self::CONTENTS_SHIFT
    }

    #[inline]
    pub const fn read_exec_only(&self) -> u32 {
        (self.bits & Self::READ_EXEC_ONLY_MASK) >> Self::READ_EXEC_ONLY_SHIFT
    }

    #[inline]
    pub const fn limit_in_pages(&self) -> u32 {
        (self.bits & Self::LIMIT_IN_PAGES_MASK) >> Self::LIMIT_IN_PAGES_SHIFT
    }

    #[inline]
    pub const fn seg_not_present(&self) -> u32 {
        (self.bits & Self::SEG_NOT_PRESENT_MASK) >> Self::SEG_NOT_PRESENT_SHIFT
    }

    #[inline]
    pub const fn useable(&self) -> u32 {
        (self.bits & Self::USEABLE_MASK) >> Self::USEABLE_SHIFT
    }

    #[inline]
    pub const fn lm(&self) -> u32 {
        (self.bits & Self::LM_MASK) >> Self::LM_SHIFT
    }

    #[inline]
    pub fn set_seg_32bit(&mut self, value: u32) {
        self.bits = (self.bits & !Self::SEG_32BIT_MASK)
            | ((value << Self::SEG_32BIT_SHIFT) & Self::SEG_32BIT_MASK);
    }

    #[inline]
    pub fn set_contents(&mut self, value: u32) {
        self.bits = (self.bits & !Self::CONTENTS_MASK)
            | ((value << Self::CONTENTS_SHIFT) & Self::CONTENTS_MASK);
    }

    #[inline]
    pub fn set_read_exec_only(&mut self, value: u32) {
        self.bits = (self.bits & !Self::READ_EXEC_ONLY_MASK)
            | ((value << Self::READ_EXEC_ONLY_SHIFT) & Self::READ_EXEC_ONLY_MASK);
    }

    #[inline]
    pub fn set_limit_in_pages(&mut self, value: u32) {
        self.bits = (self.bits & !Self::LIMIT_IN_PAGES_MASK)
            | ((value << Self::LIMIT_IN_PAGES_SHIFT) & Self::LIMIT_IN_PAGES_MASK);
    }

    #[inline]
    pub fn set_seg_not_present(&mut self, value: u32) {
        self.bits = (self.bits & !Self::SEG_NOT_PRESENT_MASK)
            | ((value << Self::SEG_NOT_PRESENT_SHIFT) & Self::SEG_NOT_PRESENT_MASK);
    }

    #[inline]
    pub fn set_useable(&mut self, value: u32) {
        self.bits = (self.bits & !Self::USEABLE_MASK)
            | ((value << Self::USEABLE_SHIFT) & Self::USEABLE_MASK);
    }

    #[inline]
    pub fn set_lm(&mut self, value: u32) {
        self.bits = (self.bits & !Self::LM_MASK)
            | ((value << Self::LM_SHIFT) & Self::LM_MASK);
    }
}

pub const MODIFY_LDT_CONTENTS_DATA: i32 = 0;
pub const MODIFY_LDT_CONTENTS_STACK: i32 = 1;
pub const MODIFY_LDT_CONTENTS_CODE: i32 = 2;
