// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: arch/x86/include/uapi/asm/ldt.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: x86_64
//! rewrite-task: S000779

/* Maximum number of LDT entries supported. */
pub const LDT_ENTRIES: u32 = 8192;

/* The size of each LDT entry. */
pub const LDT_ENTRY_SIZE: u32 = 8;

/*
 * The C bit-fields occupy one unsigned-int allocation unit after the three
 * preceding unsigned-int members.  Keep that unit as a u32 so the x86_64
 * UAPI layout and modify_ldt ABI remain unchanged.  The accessors below use
 * the allocation unit's least-significant-bit ordering used by x86 C ABI.
 */
#[repr(C)]
pub struct user_desc {
    pub entry_number: u32,
    pub base_addr: u32,
    pub limit: u32,
    pub flags: u32,
}

impl user_desc {
    pub const SEG_32BIT_SHIFT: u32 = 0;
    pub const CONTENTS_SHIFT: u32 = 1;
    pub const READ_EXEC_ONLY_SHIFT: u32 = 3;
    pub const LIMIT_IN_PAGES_SHIFT: u32 = 4;
    pub const SEG_NOT_PRESENT_SHIFT: u32 = 5;
    pub const USEABLE_SHIFT: u32 = 6;
    pub const LM_SHIFT: u32 = 7;

    pub const SEG_32BIT_MASK: u32 = 1 << Self::SEG_32BIT_SHIFT;
    pub const CONTENTS_MASK: u32 = 0b11 << Self::CONTENTS_SHIFT;
    pub const READ_EXEC_ONLY_MASK: u32 = 1 << Self::READ_EXEC_ONLY_SHIFT;
    pub const LIMIT_IN_PAGES_MASK: u32 = 1 << Self::LIMIT_IN_PAGES_SHIFT;
    pub const SEG_NOT_PRESENT_MASK: u32 = 1 << Self::SEG_NOT_PRESENT_SHIFT;
    pub const USEABLE_MASK: u32 = 1 << Self::USEABLE_SHIFT;
    pub const LM_MASK: u32 = 1 << Self::LM_SHIFT;

    pub const fn seg_32bit(&self) -> u32 {
        (self.flags & Self::SEG_32BIT_MASK) >> Self::SEG_32BIT_SHIFT
    }

    pub const fn contents(&self) -> u32 {
        (self.flags & Self::CONTENTS_MASK) >> Self::CONTENTS_SHIFT
    }

    pub const fn read_exec_only(&self) -> u32 {
        (self.flags & Self::READ_EXEC_ONLY_MASK) >> Self::READ_EXEC_ONLY_SHIFT
    }

    pub const fn limit_in_pages(&self) -> u32 {
        (self.flags & Self::LIMIT_IN_PAGES_MASK) >> Self::LIMIT_IN_PAGES_SHIFT
    }

    pub const fn seg_not_present(&self) -> u32 {
        (self.flags & Self::SEG_NOT_PRESENT_MASK) >> Self::SEG_NOT_PRESENT_SHIFT
    }

    pub const fn useable(&self) -> u32 {
        (self.flags & Self::USEABLE_MASK) >> Self::USEABLE_SHIFT
    }

    pub const fn lm(&self) -> u32 {
        (self.flags & Self::LM_MASK) >> Self::LM_SHIFT
    }
}

pub const MODIFY_LDT_CONTENTS_DATA: u32 = 0;
pub const MODIFY_LDT_CONTENTS_STACK: u32 = 1;
pub const MODIFY_LDT_CONTENTS_CODE: u32 = 2;
