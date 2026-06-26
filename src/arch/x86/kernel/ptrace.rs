//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/ptrace.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/ptrace.c
//! x86-64 register frame (pt_regs) for syscall ABI.
//! Mirrors vendor/linux/arch/x86/include/asm/ptrace.h::struct pt_regs.
//!
//! References:
//! - vendor/linux/arch/x86/kernel/ptrace.c
//! - vendor/linux/arch/x86/kernel/step.c
//! Layout and field order must match exactly — 21 u64 fields, 168 bytes total.

use core::mem::offset_of;

pub const FRAME_SIZE: usize = 168;
pub const USER_REGS_STRUCT_SIZE: usize = 27 * 8;

pub const X86_EFLAGS_CF: u64 = 1 << 0;
pub const X86_EFLAGS_PF: u64 = 1 << 2;
pub const X86_EFLAGS_AF: u64 = 1 << 4;
pub const X86_EFLAGS_ZF: u64 = 1 << 6;
pub const X86_EFLAGS_SF: u64 = 1 << 7;
pub const X86_EFLAGS_TF: u64 = 1 << 8;
pub const X86_EFLAGS_DF: u64 = 1 << 10;
pub const X86_EFLAGS_OF: u64 = 1 << 11;
pub const X86_EFLAGS_NT: u64 = 1 << 14;
pub const X86_EFLAGS_RF: u64 = 1 << 16;
pub const X86_EFLAGS_AC: u64 = 1 << 18;

/// Linux x86_64 ptrace only lets userspace change this subset of RFLAGS.
pub const USER_EFLAGS_MASK_64: u64 = X86_EFLAGS_CF
    | X86_EFLAGS_PF
    | X86_EFLAGS_AF
    | X86_EFLAGS_ZF
    | X86_EFLAGS_SF
    | X86_EFLAGS_TF
    | X86_EFLAGS_DF
    | X86_EFLAGS_OF
    | X86_EFLAGS_NT
    | X86_EFLAGS_RF
    | X86_EFLAGS_AC;

pub const USER_RPL: u64 = 3;
pub const SEGMENT_RPL_MASK: u64 = 3;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PtRegs {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbp: u64,
    pub rbx: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rax: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub orig_rax: u64,
    pub rip: u64,
    pub cs: u64,
    pub eflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

impl PtRegs {
    /// Syscall argument 0 (rdi).
    #[inline]
    pub fn arg0(&self) -> u64 {
        self.rdi
    }

    /// Syscall argument 1 (rsi).
    #[inline]
    pub fn arg1(&self) -> u64 {
        self.rsi
    }

    /// Syscall argument 2 (rdx).
    #[inline]
    pub fn arg2(&self) -> u64 {
        self.rdx
    }

    /// Syscall argument 3 (r10, NOT rcx — rcx holds rip after SYSCALL).
    #[inline]
    pub fn arg3(&self) -> u64 {
        self.r10
    }

    /// Syscall argument 4 (r8).
    #[inline]
    pub fn arg4(&self) -> u64 {
        self.r8
    }

    /// Syscall argument 5 (r9).
    #[inline]
    pub fn arg5(&self) -> u64 {
        self.r9
    }

    /// Set syscall return value in rax.
    #[inline]
    pub fn set_ret(&mut self, v: i64) {
        self.rax = v as u64;
    }
}

/// Linux x86_64 `struct user_regs_struct` as exposed through ptrace and core
/// dumps. The first 21 fields match `PtRegs`; the tail carries segment bases
/// and legacy data segment selectors.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UserRegsStruct {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub bp: u64,
    pub bx: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub ax: u64,
    pub cx: u64,
    pub dx: u64,
    pub si: u64,
    pub di: u64,
    pub orig_ax: u64,
    pub ip: u64,
    pub cs: u64,
    pub flags: u64,
    pub sp: u64,
    pub ss: u64,
    pub fs_base: u64,
    pub gs_base: u64,
    pub ds: u64,
    pub es: u64,
    pub fs: u64,
    pub gs: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SegmentState {
    pub fs_base: u64,
    pub gs_base: u64,
    pub ds: u64,
    pub es: u64,
    pub fs: u64,
    pub gs: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PtraceRegError {
    InvalidOffset,
    InvalidSelector,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SingleStepState {
    pub tif_single_step: bool,
    pub tif_forced_tf: bool,
    pub syscall_exit_trap: bool,
}

pub const REG_OFFSET_TABLE: &[(&str, usize)] = &[
    ("r15", offset_of!(PtRegs, r15)),
    ("r14", offset_of!(PtRegs, r14)),
    ("r13", offset_of!(PtRegs, r13)),
    ("r12", offset_of!(PtRegs, r12)),
    ("r11", offset_of!(PtRegs, r11)),
    ("r10", offset_of!(PtRegs, r10)),
    ("r9", offset_of!(PtRegs, r9)),
    ("r8", offset_of!(PtRegs, r8)),
    ("bx", offset_of!(PtRegs, rbx)),
    ("cx", offset_of!(PtRegs, rcx)),
    ("dx", offset_of!(PtRegs, rdx)),
    ("si", offset_of!(PtRegs, rsi)),
    ("di", offset_of!(PtRegs, rdi)),
    ("bp", offset_of!(PtRegs, rbp)),
    ("ax", offset_of!(PtRegs, rax)),
    ("orig_ax", offset_of!(PtRegs, orig_rax)),
    ("ip", offset_of!(PtRegs, rip)),
    ("cs", offset_of!(PtRegs, cs)),
    ("flags", offset_of!(PtRegs, eflags)),
    ("sp", offset_of!(PtRegs, rsp)),
    ("ss", offset_of!(PtRegs, ss)),
];

pub fn regs_query_register_offset(name: &str) -> Option<usize> {
    REG_OFFSET_TABLE
        .iter()
        .find_map(|(reg_name, offset)| (*reg_name == name).then_some(*offset))
}

pub fn regs_query_register_name(offset: usize) -> Option<&'static str> {
    REG_OFFSET_TABLE
        .iter()
        .find_map(|(reg_name, reg_offset)| (*reg_offset == offset).then_some(*reg_name))
}

pub fn read_reg_by_offset(regs: &PtRegs, offset: usize) -> Option<u64> {
    match offset {
        o if o == offset_of!(PtRegs, r15) => Some(regs.r15),
        o if o == offset_of!(PtRegs, r14) => Some(regs.r14),
        o if o == offset_of!(PtRegs, r13) => Some(regs.r13),
        o if o == offset_of!(PtRegs, r12) => Some(regs.r12),
        o if o == offset_of!(PtRegs, rbp) => Some(regs.rbp),
        o if o == offset_of!(PtRegs, rbx) => Some(regs.rbx),
        o if o == offset_of!(PtRegs, r11) => Some(regs.r11),
        o if o == offset_of!(PtRegs, r10) => Some(regs.r10),
        o if o == offset_of!(PtRegs, r9) => Some(regs.r9),
        o if o == offset_of!(PtRegs, r8) => Some(regs.r8),
        o if o == offset_of!(PtRegs, rax) => Some(regs.rax),
        o if o == offset_of!(PtRegs, rcx) => Some(regs.rcx),
        o if o == offset_of!(PtRegs, rdx) => Some(regs.rdx),
        o if o == offset_of!(PtRegs, rsi) => Some(regs.rsi),
        o if o == offset_of!(PtRegs, rdi) => Some(regs.rdi),
        o if o == offset_of!(PtRegs, orig_rax) => Some(regs.orig_rax),
        o if o == offset_of!(PtRegs, rip) => Some(regs.rip),
        o if o == offset_of!(PtRegs, cs) => Some(regs.cs),
        o if o == offset_of!(PtRegs, eflags) => Some(regs.eflags),
        o if o == offset_of!(PtRegs, rsp) => Some(regs.rsp),
        o if o == offset_of!(PtRegs, ss) => Some(regs.ss),
        _ => None,
    }
}

pub fn write_reg_by_offset(
    regs: &mut PtRegs,
    offset: usize,
    value: u64,
) -> Result<(), PtraceRegError> {
    match offset {
        o if o == offset_of!(PtRegs, r15) => regs.r15 = value,
        o if o == offset_of!(PtRegs, r14) => regs.r14 = value,
        o if o == offset_of!(PtRegs, r13) => regs.r13 = value,
        o if o == offset_of!(PtRegs, r12) => regs.r12 = value,
        o if o == offset_of!(PtRegs, rbp) => regs.rbp = value,
        o if o == offset_of!(PtRegs, rbx) => regs.rbx = value,
        o if o == offset_of!(PtRegs, r11) => regs.r11 = value,
        o if o == offset_of!(PtRegs, r10) => regs.r10 = value,
        o if o == offset_of!(PtRegs, r9) => regs.r9 = value,
        o if o == offset_of!(PtRegs, r8) => regs.r8 = value,
        o if o == offset_of!(PtRegs, rax) => regs.rax = value,
        o if o == offset_of!(PtRegs, rcx) => regs.rcx = value,
        o if o == offset_of!(PtRegs, rdx) => regs.rdx = value,
        o if o == offset_of!(PtRegs, rsi) => regs.rsi = value,
        o if o == offset_of!(PtRegs, rdi) => regs.rdi = value,
        o if o == offset_of!(PtRegs, orig_rax) => regs.orig_rax = value,
        o if o == offset_of!(PtRegs, rip) => regs.rip = value,
        o if o == offset_of!(PtRegs, cs) => {
            validate_segment_selector(value, true)?;
            regs.cs = value;
        }
        o if o == offset_of!(PtRegs, ss) => {
            validate_segment_selector(value, true)?;
            regs.ss = value;
        }
        o if o == offset_of!(PtRegs, eflags) => {
            regs.eflags = merge_user_eflags(regs.eflags, value);
        }
        o if o == offset_of!(PtRegs, rsp) => regs.rsp = value,
        _ => return Err(PtraceRegError::InvalidOffset),
    }
    Ok(())
}

pub const fn validate_segment_selector(
    value: u64,
    must_be_present: bool,
) -> Result<(), PtraceRegError> {
    if must_be_present && value == 0 {
        return Err(PtraceRegError::InvalidSelector);
    }
    if value != 0 && (value & SEGMENT_RPL_MASK) != USER_RPL {
        return Err(PtraceRegError::InvalidSelector);
    }
    Ok(())
}

pub const fn merge_user_eflags(old_flags: u64, user_flags: u64) -> u64 {
    (old_flags & !USER_EFLAGS_MASK_64) | (user_flags & USER_EFLAGS_MASK_64)
}

pub fn user_regs_from_pt_regs(regs: &PtRegs, segments: SegmentState) -> UserRegsStruct {
    UserRegsStruct {
        r15: regs.r15,
        r14: regs.r14,
        r13: regs.r13,
        r12: regs.r12,
        bp: regs.rbp,
        bx: regs.rbx,
        r11: regs.r11,
        r10: regs.r10,
        r9: regs.r9,
        r8: regs.r8,
        ax: regs.rax,
        cx: regs.rcx,
        dx: regs.rdx,
        si: regs.rsi,
        di: regs.rdi,
        orig_ax: regs.orig_rax,
        ip: regs.rip,
        cs: regs.cs,
        flags: regs.eflags,
        sp: regs.rsp,
        ss: regs.ss,
        fs_base: segments.fs_base,
        gs_base: segments.gs_base,
        ds: segments.ds,
        es: segments.es,
        fs: segments.fs,
        gs: segments.gs,
    }
}

pub fn apply_user_regs_to_pt_regs(
    user: &UserRegsStruct,
    regs: &mut PtRegs,
) -> Result<SegmentState, PtraceRegError> {
    validate_segment_selector(user.cs, true)?;
    validate_segment_selector(user.ss, true)?;
    validate_segment_selector(user.ds, false)?;
    validate_segment_selector(user.es, false)?;
    validate_segment_selector(user.fs, false)?;
    validate_segment_selector(user.gs, false)?;

    regs.r15 = user.r15;
    regs.r14 = user.r14;
    regs.r13 = user.r13;
    regs.r12 = user.r12;
    regs.rbp = user.bp;
    regs.rbx = user.bx;
    regs.r11 = user.r11;
    regs.r10 = user.r10;
    regs.r9 = user.r9;
    regs.r8 = user.r8;
    regs.rax = user.ax;
    regs.rcx = user.cx;
    regs.rdx = user.dx;
    regs.rsi = user.si;
    regs.rdi = user.di;
    regs.orig_rax = user.orig_ax;
    regs.rip = user.ip;
    regs.cs = user.cs;
    regs.eflags = merge_user_eflags(regs.eflags, user.flags);
    regs.rsp = user.sp;
    regs.ss = user.ss;

    Ok(SegmentState {
        fs_base: user.fs_base,
        gs_base: user.gs_base,
        ds: user.ds,
        es: user.es,
        fs: user.fs,
        gs: user.gs,
    })
}

pub fn user_enable_single_step(regs: &mut PtRegs, state: &mut SingleStepState) {
    state.tif_single_step = true;
    state.syscall_exit_trap = true;
    if regs.eflags & X86_EFLAGS_TF == 0 {
        state.tif_forced_tf = true;
    }
    regs.eflags |= X86_EFLAGS_TF;
}

pub fn user_disable_single_step(regs: &mut PtRegs, state: &mut SingleStepState) {
    state.tif_single_step = false;
    state.syscall_exit_trap = false;
    if state.tif_forced_tf {
        regs.eflags &= !X86_EFLAGS_TF;
    }
    state.tif_forced_tf = false;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_regs() -> PtRegs {
        PtRegs {
            r15: 15,
            r14: 14,
            r13: 13,
            r12: 12,
            rbp: 5,
            rbx: 4,
            r11: 11,
            r10: 10,
            r9: 9,
            r8: 8,
            rax: 0,
            rcx: 1,
            rdx: 2,
            rsi: 3,
            rdi: 6,
            orig_rax: 39,
            rip: 0x401000,
            cs: 0x33,
            eflags: 0x202,
            rsp: 0x7fff_fff0,
            ss: 0x2b,
        }
    }

    #[test]
    fn test_ptregs_size() {
        assert_eq!(core::mem::size_of::<PtRegs>(), FRAME_SIZE);
        assert_eq!(
            core::mem::size_of::<UserRegsStruct>(),
            USER_REGS_STRUCT_SIZE
        );
    }

    #[test]
    fn test_ptregs_layout() {
        assert_eq!(core::mem::offset_of!(PtRegs, r15), 0);
        assert_eq!(core::mem::offset_of!(PtRegs, r14), 8);
        assert_eq!(core::mem::offset_of!(PtRegs, r13), 16);
        assert_eq!(core::mem::offset_of!(PtRegs, r12), 24);
        assert_eq!(core::mem::offset_of!(PtRegs, rbp), 32);
        assert_eq!(core::mem::offset_of!(PtRegs, rbx), 40);
        assert_eq!(core::mem::offset_of!(PtRegs, r11), 48);
        assert_eq!(core::mem::offset_of!(PtRegs, r10), 56);
        assert_eq!(core::mem::offset_of!(PtRegs, r9), 64);
        assert_eq!(core::mem::offset_of!(PtRegs, r8), 72);
        assert_eq!(core::mem::offset_of!(PtRegs, rax), 80);
        assert_eq!(core::mem::offset_of!(PtRegs, rcx), 88);
        assert_eq!(core::mem::offset_of!(PtRegs, rdx), 96);
        assert_eq!(core::mem::offset_of!(PtRegs, rsi), 104);
        assert_eq!(core::mem::offset_of!(PtRegs, rdi), 112);
        assert_eq!(core::mem::offset_of!(PtRegs, orig_rax), 120);
        assert_eq!(core::mem::offset_of!(PtRegs, rip), 128);
        assert_eq!(core::mem::offset_of!(PtRegs, cs), 136);
        assert_eq!(core::mem::offset_of!(PtRegs, eflags), 144);
        assert_eq!(core::mem::offset_of!(PtRegs, rsp), 152);
        assert_eq!(core::mem::offset_of!(PtRegs, ss), 160);
    }

    #[test]
    fn test_ptregs_accessors() {
        let mut regs = sample_regs();
        regs.rdi = 0;
        regs.rsi = 1;
        regs.rdx = 2;
        regs.r10 = 3;
        regs.r8 = 4;
        regs.r9 = 5;
        assert_eq!(regs.arg0(), 0);
        assert_eq!(regs.arg1(), 1);
        assert_eq!(regs.arg2(), 2);
        assert_eq!(regs.arg3(), 3);
        assert_eq!(regs.arg4(), 4);
        assert_eq!(regs.arg5(), 5);

        regs.set_ret(-14i64);
        assert_eq!(regs.rax as i64, -14);
    }

    #[test]
    fn register_offset_queries_match_linux_names() {
        assert_eq!(regs_query_register_offset("r15"), Some(0));
        assert_eq!(regs_query_register_offset("orig_ax"), Some(120));
        assert_eq!(regs_query_register_offset("flags"), Some(144));
        assert_eq!(regs_query_register_name(152), Some("sp"));
        assert_eq!(regs_query_register_name(999), None);
    }

    #[test]
    fn read_write_by_offset_validates_segments_and_flags() {
        let mut regs = sample_regs();
        let rax = offset_of!(PtRegs, rax);
        assert_eq!(read_reg_by_offset(&regs, rax), Some(0));
        write_reg_by_offset(&mut regs, rax, 123).unwrap();
        assert_eq!(regs.rax, 123);

        let cs = offset_of!(PtRegs, cs);
        assert_eq!(
            write_reg_by_offset(&mut regs, cs, 0x10),
            Err(PtraceRegError::InvalidSelector)
        );
        write_reg_by_offset(&mut regs, cs, 0x33).unwrap();

        let flags = offset_of!(PtRegs, eflags);
        regs.eflags = 1 << 63;
        write_reg_by_offset(&mut regs, flags, X86_EFLAGS_TF | X86_EFLAGS_CF).unwrap();
        assert_eq!(regs.eflags & (1 << 63), 1 << 63);
        assert_ne!(regs.eflags & X86_EFLAGS_TF, 0);
        assert_ne!(regs.eflags & X86_EFLAGS_CF, 0);
    }

    #[test]
    fn user_regs_round_trip_preserves_ptrace_tail_segments() {
        let mut regs = sample_regs();
        let segments = SegmentState {
            fs_base: 0x7000,
            gs_base: 0x8000,
            ds: 0,
            es: 0,
            fs: 0,
            gs: 0,
        };
        let user = user_regs_from_pt_regs(&regs, segments);
        assert_eq!(user.ip, regs.rip);
        assert_eq!(user.fs_base, 0x7000);

        regs.rax = 999;
        let restored_segments = apply_user_regs_to_pt_regs(&user, &mut regs).unwrap();
        assert_eq!(regs.rax, user.ax);
        assert_eq!(regs.rip, 0x401000);
        assert_eq!(restored_segments, segments);
    }

    #[test]
    fn user_regs_reject_kernel_privilege_selectors() {
        let mut regs = sample_regs();
        let mut user = user_regs_from_pt_regs(&regs, SegmentState::default());
        user.cs = 0x10;
        assert_eq!(
            apply_user_regs_to_pt_regs(&user, &mut regs),
            Err(PtraceRegError::InvalidSelector)
        );
        user.cs = 0;
        assert_eq!(
            apply_user_regs_to_pt_regs(&user, &mut regs),
            Err(PtraceRegError::InvalidSelector)
        );
    }

    #[test]
    fn single_step_sets_and_clears_forced_trap_flag() {
        let mut regs = sample_regs();
        regs.eflags &= !X86_EFLAGS_TF;
        let mut state = SingleStepState::default();
        user_enable_single_step(&mut regs, &mut state);
        assert!(state.tif_single_step);
        assert!(state.tif_forced_tf);
        assert!(state.syscall_exit_trap);
        assert_ne!(regs.eflags & X86_EFLAGS_TF, 0);

        user_disable_single_step(&mut regs, &mut state);
        assert!(!state.tif_single_step);
        assert_eq!(regs.eflags & X86_EFLAGS_TF, 0);
    }

    #[test]
    fn single_step_preserves_user_owned_trap_flag() {
        let mut regs = sample_regs();
        regs.eflags |= X86_EFLAGS_TF;
        let mut state = SingleStepState::default();
        user_enable_single_step(&mut regs, &mut state);
        assert!(!state.tif_forced_tf);
        user_disable_single_step(&mut regs, &mut state);
        assert_ne!(regs.eflags & X86_EFLAGS_TF, 0);
    }
}
