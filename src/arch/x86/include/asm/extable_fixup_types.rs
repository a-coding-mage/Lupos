// SPDX-License-Identifier: GPL-2.0
//! linux-source: arch/x86/include/asm/extable_fixup_types.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: x86_64
//! rewrite-task: S000525

//! x86 exception-table fixup type, register, flag, and immediate encodings.
//!
//! The immediate mask retains the unsigned type of its C literal; its bit
//! pattern, rather than Rust signedness, is the contract with the assembly
//! exception-table emitters and fixup code.

pub const EX_DATA_TYPE_MASK: i32 = 0x0000_00ff;
pub const EX_DATA_REG_MASK: i32 = 0x0000_0f00;
pub const EX_DATA_FLAG_MASK: i32 = 0x0000_f000;
pub const EX_DATA_IMM_MASK: u32 = 0xffff_0000;

pub const EX_DATA_REG_SHIFT: u32 = 8;
pub const EX_DATA_FLAG_SHIFT: u32 = 12;
pub const EX_DATA_IMM_SHIFT: u32 = 16;

/*
 * Keep these as expression macros.  Like their C counterparts, they evaluate
 * the operand once and leave its integer conversion and result width to the
 * caller's expression; no helper narrows it to i32 before the shift.
 */
macro_rules! EX_DATA_REG {
    ($reg:expr) => {
        (($reg) << 8)
    };
}
pub(crate) use EX_DATA_REG;

macro_rules! EX_DATA_FLAG {
    ($flag:expr) => {
        (($flag) << 12)
    };
}
pub(crate) use EX_DATA_FLAG;

macro_rules! EX_DATA_IMM {
    ($imm:expr) => {
        (($imm) << 16)
    };
}
pub(crate) use EX_DATA_IMM;

/* segment regs */
pub const EX_REG_DS: i32 = EX_DATA_REG!(8);
pub const EX_REG_ES: i32 = EX_DATA_REG!(9);
pub const EX_REG_FS: i32 = EX_DATA_REG!(10);
pub const EX_REG_GS: i32 = EX_DATA_REG!(11);

/* flags */
pub const EX_FLAG_CLEAR_AX: i32 = EX_DATA_FLAG!(1);
pub const EX_FLAG_CLEAR_DX: i32 = EX_DATA_FLAG!(2);
pub const EX_FLAG_CLEAR_AX_DX: i32 = EX_DATA_FLAG!(3);

/* types */
pub const EX_TYPE_NONE: i32 = 0;
pub const EX_TYPE_DEFAULT: i32 = 1;
pub const EX_TYPE_FAULT: i32 = 2;
pub const EX_TYPE_UACCESS: i32 = 3;
/* unused, was: EX_TYPE_COPY = 4 */
pub const EX_TYPE_CLEAR_FS: i32 = 5;
pub const EX_TYPE_FPU_RESTORE: i32 = 6;
pub const EX_TYPE_BPF: i32 = 7;
pub const EX_TYPE_WRMSR: i32 = 8;
pub const EX_TYPE_RDMSR: i32 = 9;
pub const EX_TYPE_WRMSR_SAFE: i32 = 10; /* reg := -EIO */
pub const EX_TYPE_RDMSR_SAFE: i32 = 11; /* reg := -EIO */
pub const EX_TYPE_WRMSR_IN_MCE: i32 = 12;
pub const EX_TYPE_RDMSR_IN_MCE: i32 = 13;
pub const EX_TYPE_DEFAULT_MCE_SAFE: i32 = 14;
pub const EX_TYPE_FAULT_MCE_SAFE: i32 = 15;

pub const EX_TYPE_POP_REG: i32 = 16; /* sp += sizeof(long) */
pub const EX_TYPE_POP_ZERO: i32 = EX_TYPE_POP_REG | EX_DATA_IMM!(0);

pub const EX_TYPE_IMM_REG: i32 = 17; /* reg := (long)imm */
pub const EX_TYPE_EFAULT_REG: i32 = EX_TYPE_IMM_REG | EX_DATA_IMM!(-14); /* EFAULT */
pub const EX_TYPE_ZERO_REG: i32 = EX_TYPE_IMM_REG | EX_DATA_IMM!(0);
pub const EX_TYPE_ONE_REG: i32 = EX_TYPE_IMM_REG | EX_DATA_IMM!(1);

pub const EX_TYPE_FAULT_SGX: i32 = 18;

pub const EX_TYPE_UCOPY_LEN: i32 = 19; /* cx := reg + imm*cx */
pub const EX_TYPE_UCOPY_LEN1: i32 = EX_TYPE_UCOPY_LEN | EX_DATA_IMM!(1);
pub const EX_TYPE_UCOPY_LEN4: i32 = EX_TYPE_UCOPY_LEN | EX_DATA_IMM!(4);
pub const EX_TYPE_UCOPY_LEN8: i32 = EX_TYPE_UCOPY_LEN | EX_DATA_IMM!(8);

pub const EX_TYPE_ZEROPAD: i32 = 20; /* longword load with zeropad on fault */

pub const EX_TYPE_ERETU: i32 = 21;
