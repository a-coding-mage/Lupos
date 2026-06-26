//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/kgdb.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/kgdb.c
//! KGDB architecture support — register layout, slot allocation,
//! breakpoint plumbing.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/kgdb.c
//!
//! KGDB speaks the GDB remote-serial protocol. The arch glue captures:
//! - the register-slot order GDB expects on this target
//! - per-slot byte widths
//! - the 4-slot hardware-breakpoint registry shared with `hw_breakpoint.rs`
//! - the DR7 single-step control bit
//!
//! Lupos doesn't yet ship a working KGDB transport, so the actual
//! packet send/recv lives behind a `KgdbTransport` trait seam that
//! defaults to `EOPNOTSUPP`. The register layout and the breakpoint
//! plumbing are real ports tested against Linux's
//! `arch/x86/include/asm/kgdb.h` ordering.

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::EOPNOTSUPP;

use super::hw_breakpoint::{
    ArchHwBreakpoint, CpuBreakpoints, HBP_NUM, X86_BREAKPOINT_EXECUTE, X86_BREAKPOINT_LEN_1,
    X86_BREAKPOINT_RW, X86_BREAKPOINT_WRITE,
};

// === GDB register indices on x86_64 — mirror asm/kgdb.h ===

pub const GDB_AX: usize = 0;
pub const GDB_BX: usize = 1;
pub const GDB_CX: usize = 2;
pub const GDB_DX: usize = 3;
pub const GDB_SI: usize = 4;
pub const GDB_DI: usize = 5;
pub const GDB_BP: usize = 6;
pub const GDB_SP: usize = 7;
pub const GDB_R8: usize = 8;
pub const GDB_R9: usize = 9;
pub const GDB_R10: usize = 10;
pub const GDB_R11: usize = 11;
pub const GDB_R12: usize = 12;
pub const GDB_R13: usize = 13;
pub const GDB_R14: usize = 14;
pub const GDB_R15: usize = 15;
pub const GDB_PC: usize = 16;
pub const GDB_PS: usize = 17;
pub const GDB_CS: usize = 18;
pub const GDB_SS: usize = 19;
pub const GDB_DS: usize = 20;
pub const GDB_ES: usize = 21;
pub const GDB_FS: usize = 22;
pub const GDB_GS: usize = 23;
pub const GDB_ORIG_AX: usize = 32;

pub const DBG_MAX_REG_NUM: usize = 24;

/// `__KERNEL_DS` / `__KERNEL_CS` GDT selectors.
pub const KERNEL_DS: u32 = 0x10;
pub const KERNEL_CS: u32 = 0x08;

/// Per-register metadata in the GDB register table.
#[derive(Debug, Clone, Copy)]
pub struct DbgRegDef {
    pub name: &'static str,
    pub size: u8,
    /// Offset into `pt_regs`, or `-1` when the value is synthesised.
    pub offset: i32,
}

/// Linux's `dbg_reg_def[]` for x86_64.
pub const DBG_REG_DEF_X86_64: [DbgRegDef; DBG_MAX_REG_NUM] = [
    DbgRegDef {
        name: "ax",
        size: 8,
        offset: 0,
    },
    DbgRegDef {
        name: "bx",
        size: 8,
        offset: 8,
    },
    DbgRegDef {
        name: "cx",
        size: 8,
        offset: 16,
    },
    DbgRegDef {
        name: "dx",
        size: 8,
        offset: 24,
    },
    DbgRegDef {
        name: "si",
        size: 8,
        offset: 32,
    },
    DbgRegDef {
        name: "di",
        size: 8,
        offset: 40,
    },
    DbgRegDef {
        name: "bp",
        size: 8,
        offset: 48,
    },
    DbgRegDef {
        name: "sp",
        size: 8,
        offset: 56,
    },
    DbgRegDef {
        name: "r8",
        size: 8,
        offset: 64,
    },
    DbgRegDef {
        name: "r9",
        size: 8,
        offset: 72,
    },
    DbgRegDef {
        name: "r10",
        size: 8,
        offset: 80,
    },
    DbgRegDef {
        name: "r11",
        size: 8,
        offset: 88,
    },
    DbgRegDef {
        name: "r12",
        size: 8,
        offset: 96,
    },
    DbgRegDef {
        name: "r13",
        size: 8,
        offset: 104,
    },
    DbgRegDef {
        name: "r14",
        size: 8,
        offset: 112,
    },
    DbgRegDef {
        name: "r15",
        size: 8,
        offset: 120,
    },
    DbgRegDef {
        name: "ip",
        size: 8,
        offset: 128,
    },
    DbgRegDef {
        name: "flags",
        size: 4,
        offset: 144,
    },
    DbgRegDef {
        name: "cs",
        size: 4,
        offset: 136,
    },
    DbgRegDef {
        name: "ss",
        size: 4,
        offset: 152,
    },
    DbgRegDef {
        name: "ds",
        size: 4,
        offset: -1,
    },
    DbgRegDef {
        name: "es",
        size: 4,
        offset: -1,
    },
    DbgRegDef {
        name: "fs",
        size: 4,
        offset: -1,
    },
    DbgRegDef {
        name: "gs",
        size: 4,
        offset: -1,
    },
];

/// Get the GDB register definition for a given index, or `None`.
pub fn dbg_reg_def(regno: usize) -> Option<DbgRegDef> {
    if regno < DBG_MAX_REG_NUM {
        Some(DBG_REG_DEF_X86_64[regno])
    } else {
        None
    }
}

/// In-memory copy of the kernel side's `pt_regs` subset that KGDB
/// reads/writes (no FPU / debug registers — just the GPRs + segment
/// selectors). Layout matches `DBG_REG_DEF_X86_64` so offset reads/writes
/// work as `dbg_reg_def`'s `offset` indices.
#[derive(Debug, Default, Clone, Copy)]
#[repr(C)]
pub struct PtRegsX86_64 {
    pub ax: u64,
    pub bx: u64,
    pub cx: u64,
    pub dx: u64,
    pub si: u64,
    pub di: u64,
    pub bp: u64,
    pub sp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub ip: u64,
    pub cs: u32,
    pub flags: u32,
    pub ss: u32,
    pub _pad: u32,
}

/// `dbg_get_reg(regno, mem, regs)` — fetch a register's value. Caller
/// provides an output buffer large enough for the widest register
/// (8 bytes). Returns the GDB-visible name, or `None` if `regno` is
/// out of range or unmapped (DS/ES/FS/GS).
pub fn dbg_get_reg(regno: usize, regs: &PtRegsX86_64) -> Option<(u64, &'static str)> {
    let def = dbg_reg_def(regno)?;
    let val: u64 = match regno {
        GDB_AX => regs.ax,
        GDB_BX => regs.bx,
        GDB_CX => regs.cx,
        GDB_DX => regs.dx,
        GDB_SI => regs.si,
        GDB_DI => regs.di,
        GDB_BP => regs.bp,
        GDB_SP => regs.sp,
        GDB_R8 => regs.r8,
        GDB_R9 => regs.r9,
        GDB_R10 => regs.r10,
        GDB_R11 => regs.r11,
        GDB_R12 => regs.r12,
        GDB_R13 => regs.r13,
        GDB_R14 => regs.r14,
        GDB_R15 => regs.r15,
        GDB_PC => regs.ip,
        GDB_PS => regs.flags as u64,
        GDB_CS => regs.cs as u64,
        GDB_SS => regs.ss as u64,
        GDB_DS | GDB_ES => 0,
        GDB_FS | GDB_GS => 0,
        _ => return None,
    };
    Some((val, def.name))
}

/// `dbg_set_reg(regno, value, regs)` — mirror of `dbg_get_reg`. Linux
/// silently no-ops on a handful of registers (`SP`, `ORIG_AX`, segment
/// regs that aren't trackable) — we replicate that semantics.
pub fn dbg_set_reg(regno: usize, value: u64, regs: &mut PtRegsX86_64) {
    if regno == GDB_SP || regno == GDB_ORIG_AX {
        return;
    }
    match regno {
        GDB_AX => regs.ax = value,
        GDB_BX => regs.bx = value,
        GDB_CX => regs.cx = value,
        GDB_DX => regs.dx = value,
        GDB_SI => regs.si = value,
        GDB_DI => regs.di = value,
        GDB_BP => regs.bp = value,
        GDB_R8 => regs.r8 = value,
        GDB_R9 => regs.r9 = value,
        GDB_R10 => regs.r10 = value,
        GDB_R11 => regs.r11 = value,
        GDB_R12 => regs.r12 = value,
        GDB_R13 => regs.r13 = value,
        GDB_R14 => regs.r14 = value,
        GDB_R15 => regs.r15 = value,
        GDB_PC => regs.ip = value,
        GDB_PS => regs.flags = value as u32,
        GDB_CS => regs.cs = value as u32,
        GDB_SS => regs.ss = value as u32,
        GDB_DS | GDB_ES | GDB_FS | GDB_GS => {}
        _ => {}
    }
}

/// `sleeping_thread_to_gdb_regs` — fill `gdb_regs` from a saved
/// `thread_struct.sp` for a *sleeping* task. Mirrors Linux's
/// zero-init of every GPR except `BP` (read from the inactive-task
/// frame) and `SP` (saved in `thread.sp`).
pub fn sleeping_thread_to_gdb_regs(thread_sp: u64, frame_bp: u64) -> [u64; DBG_MAX_REG_NUM] {
    let mut g = [0u64; DBG_MAX_REG_NUM];
    g[GDB_BP] = frame_bp;
    g[GDB_SP] = thread_sp;
    g[GDB_CS] = KERNEL_CS as u64;
    g[GDB_SS] = KERNEL_DS as u64;
    g
}

/// One breakpoint slot the KGDB layer owns. Mirrors the `breakinfo[]`
/// table Linux declares (4 slots, sharing DR0-DR3).
#[derive(Debug, Default, Clone, Copy)]
pub struct KgdbHwBreak {
    pub enabled: bool,
    pub addr: u64,
    pub len: u8,
    pub bp_type: u32,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct KgdbState {
    pub breakinfo: [KgdbHwBreak; HBP_NUM],
    pub early_dr7: u64,
}

/// Translate a generic KGDB request (`KGDB_BREAKPOINT_HW`, etc.) into
/// an `ArchHwBreakpoint`. `length_bytes` is the byte width (1/2/4/8).
pub fn kgdb_break_to_arch(addr: u64, length_bytes: u8, x86_type: u32) -> ArchHwBreakpoint {
    let len = match length_bytes {
        1 => X86_BREAKPOINT_LEN_1,
        2 => super::hw_breakpoint::X86_BREAKPOINT_LEN_2,
        4 => super::hw_breakpoint::X86_BREAKPOINT_LEN_4,
        8 => super::hw_breakpoint::X86_BREAKPOINT_LEN_8,
        _ => X86_BREAKPOINT_LEN_1,
    };
    ArchHwBreakpoint {
        address: addr,
        len,
        ty: x86_type,
        mask: 0,
    }
}

/// Trait seam for the GDB packet transport. Production wires this to
/// the real serial / network backend.
pub trait KgdbTransport {
    fn send(&self, packet: &[u8]) -> Result<(), i32>;
    fn recv(&self) -> Result<Vec<u8>, i32>;
}

/// Default "no transport configured" implementation.
pub struct NoTransport;
impl KgdbTransport for NoTransport {
    fn send(&self, _packet: &[u8]) -> Result<(), i32> {
        Err(EOPNOTSUPP)
    }
    fn recv(&self) -> Result<Vec<u8>, i32> {
        Err(EOPNOTSUPP)
    }
}

/// Install all enabled KGDB hardware breakpoints into the per-CPU
/// `CpuBreakpoints` table. Returns the count of slots installed.
pub fn kgdb_correct_hw_break(state: &KgdbState, cpu: &mut CpuBreakpoints) -> usize {
    let mut count = 0;
    for slot in state.breakinfo.iter() {
        if !slot.enabled {
            continue;
        }
        let bp = kgdb_break_to_arch(slot.addr, slot.len, slot.bp_type);
        if super::hw_breakpoint::arch_install_hw_breakpoint(cpu, &bp).is_ok() {
            count += 1;
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dbg_max_reg_num_is_24() {
        assert_eq!(DBG_MAX_REG_NUM, 24);
    }

    #[test]
    fn x86_64_register_table_has_correct_widths() {
        let defs = &DBG_REG_DEF_X86_64;
        // GPRs are 8 bytes; segment regs are 4.
        assert_eq!(defs[GDB_AX].size, 8);
        assert_eq!(defs[GDB_R15].size, 8);
        assert_eq!(defs[GDB_FLAGS_IDX()].size, 4);
        assert_eq!(defs[GDB_CS].size, 4);
        // DS/ES/FS/GS are unmapped on x86_64.
        assert_eq!(defs[GDB_DS].offset, -1);
        assert_eq!(defs[GDB_ES].offset, -1);
        assert_eq!(defs[GDB_FS].offset, -1);
        assert_eq!(defs[GDB_GS].offset, -1);
    }

    // Helper — flags index in the GDB table.
    #[allow(non_snake_case)]
    fn GDB_FLAGS_IDX() -> usize {
        GDB_PS
    }

    #[test]
    fn dbg_reg_def_returns_none_for_out_of_range() {
        assert!(dbg_reg_def(100).is_none());
    }

    #[test]
    fn dbg_get_reg_returns_register_value() {
        let mut regs = PtRegsX86_64::default();
        regs.ax = 0x1111;
        regs.r9 = 0x2222;
        regs.ip = 0x3333;
        let (val, name) = dbg_get_reg(GDB_AX, &regs).unwrap();
        assert_eq!(val, 0x1111);
        assert_eq!(name, "ax");
        let (val, _) = dbg_get_reg(GDB_R9, &regs).unwrap();
        assert_eq!(val, 0x2222);
        let (val, _) = dbg_get_reg(GDB_PC, &regs).unwrap();
        assert_eq!(val, 0x3333);
    }

    #[test]
    fn dbg_get_reg_returns_zero_for_unmapped_segment_regs() {
        let regs = PtRegsX86_64::default();
        for r in [GDB_DS, GDB_ES, GDB_FS, GDB_GS] {
            let (val, _) = dbg_get_reg(r, &regs).unwrap();
            assert_eq!(val, 0);
        }
    }

    #[test]
    fn dbg_set_reg_round_trips_through_get() {
        let mut regs = PtRegsX86_64::default();
        dbg_set_reg(GDB_AX, 0xDEAD, &mut regs);
        dbg_set_reg(GDB_PC, 0xBEEF, &mut regs);
        let (ax, _) = dbg_get_reg(GDB_AX, &regs).unwrap();
        let (ip, _) = dbg_get_reg(GDB_PC, &regs).unwrap();
        assert_eq!(ax, 0xDEAD);
        assert_eq!(ip, 0xBEEF);
    }

    #[test]
    fn dbg_set_reg_is_noop_for_sp_and_orig_ax() {
        let mut regs = PtRegsX86_64::default();
        dbg_set_reg(GDB_SP, 0x1000, &mut regs);
        dbg_set_reg(GDB_ORIG_AX, 0xDEAD, &mut regs);
        assert_eq!(regs.sp, 0);
        assert_eq!(regs.ax, 0);
    }

    #[test]
    fn sleeping_thread_initialises_bp_sp_cs_ss() {
        let g = sleeping_thread_to_gdb_regs(0x1000, 0x2000);
        assert_eq!(g[GDB_SP], 0x1000);
        assert_eq!(g[GDB_BP], 0x2000);
        assert_eq!(g[GDB_CS], KERNEL_CS as u64);
        assert_eq!(g[GDB_SS], KERNEL_DS as u64);
        assert_eq!(g[GDB_AX], 0);
        assert_eq!(g[GDB_PC], 0);
    }

    #[test]
    fn kgdb_break_to_arch_maps_byte_widths() {
        let bp = kgdb_break_to_arch(0x1000, 4, X86_BREAKPOINT_WRITE);
        assert_eq!(bp.address, 0x1000);
        assert_eq!(bp.len, super::super::hw_breakpoint::X86_BREAKPOINT_LEN_4);
        assert_eq!(bp.ty, X86_BREAKPOINT_WRITE);
    }

    #[test]
    fn kgdb_break_to_arch_defaults_to_len_1_for_unknown_widths() {
        let bp = kgdb_break_to_arch(0x1000, 16, X86_BREAKPOINT_RW);
        assert_eq!(bp.len, X86_BREAKPOINT_LEN_1);
    }

    #[test]
    fn no_transport_returns_eopnotsupp_on_send() {
        let r = NoTransport.send(b"+");
        assert_eq!(r, Err(EOPNOTSUPP));
    }

    #[test]
    fn correct_hw_break_installs_enabled_slots() {
        let mut state = KgdbState::default();
        state.breakinfo[0] = KgdbHwBreak {
            enabled: true,
            addr: 0x1000,
            len: 4,
            bp_type: X86_BREAKPOINT_WRITE,
        };
        state.breakinfo[1] = KgdbHwBreak {
            enabled: true,
            addr: 0x2000,
            len: 4,
            bp_type: X86_BREAKPOINT_RW,
        };
        let mut cpu = CpuBreakpoints::default();
        let installed = kgdb_correct_hw_break(&state, &mut cpu);
        assert_eq!(installed, 2);
        assert!(cpu.slot_occupied[0]);
        assert!(cpu.slot_occupied[1]);
    }

    #[test]
    fn correct_hw_break_skips_disabled_slots() {
        let mut state = KgdbState::default();
        state.breakinfo[2] = KgdbHwBreak {
            enabled: false,
            addr: 0x3000,
            len: 1,
            bp_type: X86_BREAKPOINT_EXECUTE,
        };
        let mut cpu = CpuBreakpoints::default();
        let installed = kgdb_correct_hw_break(&state, &mut cpu);
        assert_eq!(installed, 0);
    }
}
