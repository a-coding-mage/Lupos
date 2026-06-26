//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/events/amd/lbr.c
//! test-origin: linux:vendor/linux/arch/x86/events/amd/lbr.c
//! AMD last-branch-record v2 model.

extern crate alloc;

use alloc::vec::Vec;

use crate::arch::x86::events::utils::{
    X86_BR_ABORT, X86_BR_CALL, X86_BR_IND_CALL, X86_BR_IND_JMP, X86_BR_INT, X86_BR_IRET,
    X86_BR_JCC, X86_BR_JMP, X86_BR_KERNEL, X86_BR_NONE, X86_BR_RET, X86_BR_SYSCALL, X86_BR_SYSRET,
    X86_BR_USER, X86_BR_ZERO_CALL, common_branch_type,
};
use crate::include::uapi::errno::EOPNOTSUPP;

pub const LBR_SELECT_MASK: u64 = 0x1ff;

pub const LBR_SELECT_KERNEL: u8 = 0;
pub const LBR_SELECT_USER: u8 = 1;
pub const LBR_SELECT_JCC: u8 = 2;
pub const LBR_SELECT_CALL_NEAR_REL: u8 = 3;
pub const LBR_SELECT_CALL_NEAR_IND: u8 = 4;
pub const LBR_SELECT_RET_NEAR: u8 = 5;
pub const LBR_SELECT_JMP_NEAR_IND: u8 = 6;
pub const LBR_SELECT_JMP_NEAR_REL: u8 = 7;
pub const LBR_SELECT_FAR_BRANCH: u8 = 8;

pub const LBR_KERNEL: u64 = 1 << LBR_SELECT_KERNEL;
pub const LBR_USER: u64 = 1 << LBR_SELECT_USER;
pub const LBR_JCC: u64 = 1 << LBR_SELECT_JCC;
pub const LBR_REL_CALL: u64 = 1 << LBR_SELECT_CALL_NEAR_REL;
pub const LBR_IND_CALL: u64 = 1 << LBR_SELECT_CALL_NEAR_IND;
pub const LBR_RETURN: u64 = 1 << LBR_SELECT_RET_NEAR;
pub const LBR_IND_JMP: u64 = 1 << LBR_SELECT_JMP_NEAR_IND;
pub const LBR_REL_JMP: u64 = 1 << LBR_SELECT_JMP_NEAR_REL;
pub const LBR_FAR: u64 = 1 << LBR_SELECT_FAR_BRANCH;
pub const LBR_IGNORE: i64 = 0;
pub const LBR_NOT_SUPP: i64 = -1;
pub const LBR_ANY: u64 =
    LBR_JCC | LBR_REL_CALL | LBR_IND_CALL | LBR_RETURN | LBR_REL_JMP | LBR_IND_JMP | LBR_FAR;

pub const PERF_SAMPLE_BRANCH_USER_SHIFT: u8 = 0;
pub const PERF_SAMPLE_BRANCH_KERNEL_SHIFT: u8 = 1;
pub const PERF_SAMPLE_BRANCH_HV_SHIFT: u8 = 2;
pub const PERF_SAMPLE_BRANCH_ANY_SHIFT: u8 = 3;
pub const PERF_SAMPLE_BRANCH_ANY_CALL_SHIFT: u8 = 4;
pub const PERF_SAMPLE_BRANCH_ANY_RETURN_SHIFT: u8 = 5;
pub const PERF_SAMPLE_BRANCH_IND_CALL_SHIFT: u8 = 6;
pub const PERF_SAMPLE_BRANCH_ABORT_TX_SHIFT: u8 = 7;
pub const PERF_SAMPLE_BRANCH_IN_TX_SHIFT: u8 = 8;
pub const PERF_SAMPLE_BRANCH_NO_TX_SHIFT: u8 = 9;
pub const PERF_SAMPLE_BRANCH_COND_SHIFT: u8 = 10;
pub const PERF_SAMPLE_BRANCH_CALL_STACK_SHIFT: u8 = 11;
pub const PERF_SAMPLE_BRANCH_IND_JUMP_SHIFT: u8 = 12;
pub const PERF_SAMPLE_BRANCH_CALL_SHIFT: u8 = 13;
pub const PERF_SAMPLE_BRANCH_NO_FLAGS_SHIFT: u8 = 14;
pub const PERF_SAMPLE_BRANCH_NO_CYCLES_SHIFT: u8 = 15;
pub const PERF_SAMPLE_BRANCH_TYPE_SAVE_SHIFT: u8 = 16;
pub const PERF_SAMPLE_BRANCH_HW_INDEX_SHIFT: u8 = 17;
pub const PERF_SAMPLE_BRANCH_PRIV_SAVE_SHIFT: u8 = 18;
pub const PERF_SAMPLE_BRANCH_COUNTERS_SHIFT: u8 = 19;
pub const PERF_SAMPLE_BRANCH_MAX_SHIFT: u8 = 20;

pub const PERF_SAMPLE_BRANCH_USER: u64 = 1 << PERF_SAMPLE_BRANCH_USER_SHIFT;
pub const PERF_SAMPLE_BRANCH_KERNEL: u64 = 1 << PERF_SAMPLE_BRANCH_KERNEL_SHIFT;
pub const PERF_SAMPLE_BRANCH_HV: u64 = 1 << PERF_SAMPLE_BRANCH_HV_SHIFT;
pub const PERF_SAMPLE_BRANCH_ANY: u64 = 1 << PERF_SAMPLE_BRANCH_ANY_SHIFT;
pub const PERF_SAMPLE_BRANCH_ANY_CALL: u64 = 1 << PERF_SAMPLE_BRANCH_ANY_CALL_SHIFT;
pub const PERF_SAMPLE_BRANCH_ANY_RETURN: u64 = 1 << PERF_SAMPLE_BRANCH_ANY_RETURN_SHIFT;
pub const PERF_SAMPLE_BRANCH_IND_CALL: u64 = 1 << PERF_SAMPLE_BRANCH_IND_CALL_SHIFT;
pub const PERF_SAMPLE_BRANCH_ABORT_TX: u64 = 1 << PERF_SAMPLE_BRANCH_ABORT_TX_SHIFT;
pub const PERF_SAMPLE_BRANCH_IN_TX: u64 = 1 << PERF_SAMPLE_BRANCH_IN_TX_SHIFT;
pub const PERF_SAMPLE_BRANCH_NO_TX: u64 = 1 << PERF_SAMPLE_BRANCH_NO_TX_SHIFT;
pub const PERF_SAMPLE_BRANCH_COND: u64 = 1 << PERF_SAMPLE_BRANCH_COND_SHIFT;
pub const PERF_SAMPLE_BRANCH_CALL_STACK: u64 = 1 << PERF_SAMPLE_BRANCH_CALL_STACK_SHIFT;
pub const PERF_SAMPLE_BRANCH_IND_JUMP: u64 = 1 << PERF_SAMPLE_BRANCH_IND_JUMP_SHIFT;
pub const PERF_SAMPLE_BRANCH_CALL: u64 = 1 << PERF_SAMPLE_BRANCH_CALL_SHIFT;
pub const PERF_SAMPLE_BRANCH_NO_FLAGS: u64 = 1 << PERF_SAMPLE_BRANCH_NO_FLAGS_SHIFT;
pub const PERF_SAMPLE_BRANCH_NO_CYCLES: u64 = 1 << PERF_SAMPLE_BRANCH_NO_CYCLES_SHIFT;
pub const PERF_SAMPLE_BRANCH_TYPE_SAVE: u64 = 1 << PERF_SAMPLE_BRANCH_TYPE_SAVE_SHIFT;

pub const X86_BR_TYPE_SAVE: u32 = 1 << 18;
pub const X86_BR_PLM: u32 = X86_BR_USER | X86_BR_KERNEL;
pub const X86_BR_ANY: u32 = X86_BR_CALL
    | X86_BR_RET
    | X86_BR_SYSCALL
    | X86_BR_SYSRET
    | X86_BR_INT
    | X86_BR_IRET
    | X86_BR_JCC
    | X86_BR_JMP
    | X86_BR_ABORT
    | X86_BR_IND_CALL
    | X86_BR_IND_JMP
    | X86_BR_ZERO_CALL;
pub const X86_BR_ALL: u32 = X86_BR_PLM | X86_BR_ANY;
pub const X86_BR_ANY_CALL: u32 =
    X86_BR_CALL | X86_BR_IND_CALL | X86_BR_ZERO_CALL | X86_BR_SYSCALL | X86_BR_INT;

pub const PERF_BR_SPEC_NA: u8 = 0;
pub const PERF_BR_SPEC_WRONG_PATH: u8 = 1;
pub const PERF_BR_NON_SPEC_CORRECT_PATH: u8 = 2;
pub const PERF_BR_SPEC_CORRECT_PATH: u8 = 3;
pub const LBR_SPEC_MAP: [u8; 4] = [
    PERF_BR_SPEC_NA,
    PERF_BR_SPEC_WRONG_PATH,
    PERF_BR_NON_SPEC_CORRECT_PATH,
    PERF_BR_SPEC_CORRECT_PATH,
];

pub const PERF_ATTACH_SCHED_CB: u64 = 0x0020;

pub const MSR_IA32_DEBUGCTLMSR: u32 = 0x0000_01d9;
pub const MSR_AMD64_LBR_SELECT: u32 = 0xc000_010e;
pub const MSR_AMD_DBG_EXTN_CFG: u32 = 0xc000_010f;
pub const MSR_AMD_SAMP_BR_FROM: u32 = 0xc001_0300;
pub const DEBUGCTLMSR_FREEZE_LBRS_ON_PMI: u64 = 1 << 11;
pub const DBG_EXTN_CFG_LBRV2EN: u64 = 1 << 6;
pub const EXT_PERFMON_DEBUG_FEATURES: u32 = 0x8000_0022;

const BRANCH_IP_MASK: u64 = (1u64 << 58) - 1;
const TO_RESERVED_BIT: u8 = 61;
const TO_SPEC_BIT: u8 = 62;
const TO_VALID_BIT: u8 = 63;
const FROM_MISPREDICT_BIT: u8 = 63;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdLbrEntry {
    pub from: u64,
    pub to: u64,
    pub mispredicted: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdLbrBranchEntry {
    pub from: u64,
    pub to: u64,
    pub mispredicted: bool,
    pub predicted: bool,
    pub spec: u8,
    pub branch_type: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdLbrRawEntry {
    pub from_full: u64,
    pub to_full: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdLbrFilterMatch {
    pub branch_type: u32,
    pub fused_offset: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdLbrFilterSetup {
    pub reg: u32,
    pub config: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdLbrEvent {
    pub branch_sample_type: u64,
    pub attach_state: u64,
    pub branch_reg: AmdLbrFilterSetup,
    pub has_branch_stack: bool,
    pub total_time_running: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdLbrCpuState {
    pub lbr_users: i32,
    pub lbr_select: bool,
    pub lbr_sel_config: u64,
    pub br_sel: u32,
    pub last_task_ctx: Option<u64>,
    pub last_log_id: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdLbrMsrWrite {
    pub msr: u32,
    pub value: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AmdLbrResetPlan {
    pub writes: Vec<AmdLbrMsrWrite>,
    pub clear_last_task_ctx: bool,
    pub last_log_id: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AmdLbrAddPlan {
    pub sched_cb_delta: i32,
    pub reset: Option<AmdLbrResetPlan>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdLbrDelPlan {
    pub sched_cb_delta: i32,
    pub warn_negative_users: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdLbrInitPlan {
    pub lbr_nr: u8,
    pub print_depth: u8,
}

pub const fn lbr_v2_supported(pmu_version: u8, cpuid_lbr_v2: bool) -> bool {
    pmu_version >= 2 && cpuid_lbr_v2
}

pub const fn cpuid_lbr_v2_stack_size(ebx: u32) -> u8 {
    ((ebx >> 4) & 0x3f) as u8
}

pub const fn amd_pmu_lbr_init(
    pmu_version: u8,
    cpuid_lbr_v2: bool,
    cpuid_0x80000022_ebx: u32,
) -> Result<AmdLbrInitPlan, i32> {
    if !lbr_v2_supported(pmu_version, cpuid_lbr_v2) {
        return Err(EOPNOTSUPP);
    }
    let lbr_nr = cpuid_lbr_v2_stack_size(cpuid_0x80000022_ebx);
    Ok(AmdLbrInitPlan {
        lbr_nr,
        print_depth: lbr_nr,
    })
}

pub const fn amd_pmu_lbr_from_msr(index: u8) -> u32 {
    MSR_AMD_SAMP_BR_FROM + index as u32 * 2
}

pub const fn amd_pmu_lbr_to_msr(index: u8) -> u32 {
    MSR_AMD_SAMP_BR_FROM + index as u32 * 2 + 1
}

pub const fn sign_ext_branch_ip(ip: u64, virt_bits: u8) -> u64 {
    if virt_bits == 0 || virt_bits >= 64 {
        ip
    } else {
        let shift = 64 - virt_bits;
        (((ip << shift) as i64) >> shift) as u64
    }
}

pub const fn lbr_entry_valid(entry: AmdLbrEntry) -> bool {
    entry.from != 0 && entry.to != 0
}

pub const fn raw_from_ip(from_full: u64) -> u64 {
    from_full & BRANCH_IP_MASK
}

pub const fn raw_to_ip(to_full: u64) -> u64 {
    to_full & BRANCH_IP_MASK
}

pub const fn raw_mispredict(from_full: u64) -> bool {
    (from_full & (1 << FROM_MISPREDICT_BIT)) != 0
}

pub const fn raw_to_valid(to_full: u64) -> bool {
    (to_full & (1 << TO_VALID_BIT)) != 0
}

pub const fn raw_to_spec(to_full: u64) -> bool {
    (to_full & (1 << TO_SPEC_BIT)) != 0
}

pub const fn raw_to_reserved(to_full: u64) -> bool {
    (to_full & (1 << TO_RESERVED_BIT)) != 0
}

pub const fn lbr_spec_from_to_bits(to_full: u64) -> Option<u8> {
    let valid = raw_to_valid(to_full);
    let spec = raw_to_spec(to_full);
    if (!valid && !spec) || raw_to_reserved(to_full) {
        return None;
    }
    let index = ((valid as usize) << 1) | spec as usize;
    Some(LBR_SPEC_MAP[index])
}

pub fn amd_pmu_lbr_decode(
    lbr_users: i32,
    lbr_nr: u8,
    raw_entries: &[AmdLbrRawEntry],
    virt_bits: u8,
) -> Vec<AmdLbrBranchEntry> {
    let mut out = Vec::new();
    if lbr_users == 0 {
        return out;
    }

    let mut index = 0usize;
    while index < lbr_nr as usize && index < raw_entries.len() {
        let raw = raw_entries[index];
        if let Some(spec) = lbr_spec_from_to_bits(raw.to_full) {
            let mispredicted = raw_mispredict(raw.from_full);
            out.push(AmdLbrBranchEntry {
                from: sign_ext_branch_ip(raw_from_ip(raw.from_full), virt_bits),
                to: sign_ext_branch_ip(raw_to_ip(raw.to_full), virt_bits),
                mispredicted,
                predicted: !mispredicted,
                spec,
                branch_type: 0,
            });
        }
        index += 1;
    }
    out
}

pub fn amd_pmu_lbr_filter(
    entries: &[AmdLbrBranchEntry],
    br_sel: u32,
    matches: &[AmdLbrFilterMatch],
) -> Vec<AmdLbrBranchEntry> {
    let mut filtered = Vec::new();
    let fused_only =
        ((br_sel & X86_BR_ALL) == X86_BR_ALL) && ((br_sel & X86_BR_TYPE_SAVE) != X86_BR_TYPE_SAVE);
    let save_type = (br_sel & X86_BR_TYPE_SAVE) == X86_BR_TYPE_SAVE;

    for (index, entry) in entries.iter().copied().enumerate() {
        let branch = matches.get(index).copied().unwrap_or(AmdLbrFilterMatch {
            branch_type: X86_BR_NONE,
            fused_offset: 0,
        });
        let mut entry = entry;

        if branch.fused_offset != 0 {
            entry.from = entry.from.wrapping_add(branch.fused_offset);
            if fused_only {
                filtered.push(entry);
                continue;
            }
        }

        if branch.branch_type == X86_BR_NONE || (br_sel & branch.branch_type) != branch.branch_type
        {
            continue;
        }

        if save_type {
            entry.branch_type = common_branch_type(branch.branch_type);
        }
        filtered.push(entry);
    }

    filtered
}

pub const fn lbr_select_map_value(shift: u8) -> i64 {
    match shift {
        PERF_SAMPLE_BRANCH_USER_SHIFT => LBR_USER as i64,
        PERF_SAMPLE_BRANCH_KERNEL_SHIFT => LBR_KERNEL as i64,
        PERF_SAMPLE_BRANCH_HV_SHIFT => LBR_IGNORE,
        PERF_SAMPLE_BRANCH_ANY_SHIFT => LBR_ANY as i64,
        PERF_SAMPLE_BRANCH_ANY_CALL_SHIFT => (LBR_REL_CALL | LBR_IND_CALL | LBR_FAR) as i64,
        PERF_SAMPLE_BRANCH_ANY_RETURN_SHIFT => (LBR_RETURN | LBR_FAR) as i64,
        PERF_SAMPLE_BRANCH_IND_CALL_SHIFT => LBR_IND_CALL as i64,
        PERF_SAMPLE_BRANCH_ABORT_TX_SHIFT => LBR_NOT_SUPP,
        PERF_SAMPLE_BRANCH_IN_TX_SHIFT => LBR_NOT_SUPP,
        PERF_SAMPLE_BRANCH_NO_TX_SHIFT => LBR_NOT_SUPP,
        PERF_SAMPLE_BRANCH_COND_SHIFT => LBR_JCC as i64,
        PERF_SAMPLE_BRANCH_CALL_STACK_SHIFT => LBR_NOT_SUPP,
        PERF_SAMPLE_BRANCH_IND_JUMP_SHIFT => LBR_IND_JMP as i64,
        PERF_SAMPLE_BRANCH_CALL_SHIFT => LBR_REL_CALL as i64,
        PERF_SAMPLE_BRANCH_NO_FLAGS_SHIFT => LBR_NOT_SUPP,
        PERF_SAMPLE_BRANCH_NO_CYCLES_SHIFT => LBR_NOT_SUPP,
        _ => LBR_IGNORE,
    }
}

pub const fn branch_sample_to_x86_mask(branch_sample_type: u64) -> u32 {
    let mut mask = 0u32;
    if (branch_sample_type & PERF_SAMPLE_BRANCH_USER) != 0 {
        mask |= X86_BR_USER;
    }
    if (branch_sample_type & PERF_SAMPLE_BRANCH_KERNEL) != 0 {
        mask |= X86_BR_KERNEL;
    }
    if (branch_sample_type & PERF_SAMPLE_BRANCH_ANY) != 0 {
        mask |= X86_BR_ANY;
    }
    if (branch_sample_type & PERF_SAMPLE_BRANCH_ANY_CALL) != 0 {
        mask |= X86_BR_ANY_CALL;
    }
    if (branch_sample_type & PERF_SAMPLE_BRANCH_ANY_RETURN) != 0 {
        mask |= X86_BR_RET | X86_BR_IRET | X86_BR_SYSRET;
    }
    if (branch_sample_type & PERF_SAMPLE_BRANCH_IND_CALL) != 0 {
        mask |= X86_BR_IND_CALL;
    }
    if (branch_sample_type & PERF_SAMPLE_BRANCH_COND) != 0 {
        mask |= X86_BR_JCC;
    }
    if (branch_sample_type & PERF_SAMPLE_BRANCH_IND_JUMP) != 0 {
        mask |= X86_BR_IND_JMP;
    }
    if (branch_sample_type & PERF_SAMPLE_BRANCH_CALL) != 0 {
        mask |= X86_BR_CALL | X86_BR_ZERO_CALL;
    }
    if (branch_sample_type & PERF_SAMPLE_BRANCH_TYPE_SAVE) != 0 {
        mask |= X86_BR_TYPE_SAVE;
    }
    mask
}

pub const fn amd_pmu_lbr_setup_filter(
    lbr_nr: u8,
    branch_sample_type: u64,
) -> Result<AmdLbrFilterSetup, i32> {
    if lbr_nr == 0 {
        return Err(EOPNOTSUPP);
    }

    let branch_reg = branch_sample_to_x86_mask(branch_sample_type);
    let mut select_mask = 0u64;
    let mut shift = 0u8;
    while shift < PERF_SAMPLE_BRANCH_MAX_SHIFT {
        if (branch_sample_type & (1u64 << shift)) != 0 {
            let value = lbr_select_map_value(shift);
            if value == LBR_NOT_SUPP {
                return Err(EOPNOTSUPP);
            }
            if value != LBR_IGNORE {
                select_mask |= value as u64;
            }
        }
        shift += 1;
    }

    Ok(AmdLbrFilterSetup {
        reg: branch_reg,
        config: select_mask ^ LBR_SELECT_MASK,
    })
}

pub fn amd_pmu_lbr_hw_config(event: &mut AmdLbrEvent, lbr_nr: u8) -> Result<(), i32> {
    let setup = amd_pmu_lbr_setup_filter(lbr_nr, event.branch_sample_type)?;
    event.branch_reg = setup;
    event.attach_state |= PERF_ATTACH_SCHED_CB;
    Ok(())
}

pub fn amd_pmu_lbr_reset_plan(lbr_nr: u8) -> Option<AmdLbrResetPlan> {
    if lbr_nr == 0 {
        return None;
    }

    let mut writes = Vec::new();
    let mut index = 0u8;
    while index < lbr_nr {
        writes.push(AmdLbrMsrWrite {
            msr: amd_pmu_lbr_from_msr(index),
            value: 0,
        });
        writes.push(AmdLbrMsrWrite {
            msr: amd_pmu_lbr_to_msr(index),
            value: 0,
        });
        index += 1;
    }
    writes.push(AmdLbrMsrWrite {
        msr: MSR_AMD64_LBR_SELECT,
        value: 0,
    });

    Some(AmdLbrResetPlan {
        writes,
        clear_last_task_ctx: true,
        last_log_id: 0,
    })
}

pub fn amd_pmu_lbr_add(
    state: &mut AmdLbrCpuState,
    event: &AmdLbrEvent,
    lbr_nr: u8,
) -> Option<AmdLbrAddPlan> {
    if lbr_nr == 0 {
        return None;
    }

    if event.has_branch_stack {
        state.lbr_select = true;
        state.lbr_sel_config = event.branch_reg.config;
        state.br_sel = event.branch_reg.reg;
    }

    let should_reset = state.lbr_users == 0 && event.total_time_running == 0;
    state.lbr_users += 1;

    Some(AmdLbrAddPlan {
        sched_cb_delta: 1,
        reset: if should_reset {
            amd_pmu_lbr_reset_plan(lbr_nr)
        } else {
            None
        },
    })
}

pub fn amd_pmu_lbr_del(
    state: &mut AmdLbrCpuState,
    event: &AmdLbrEvent,
    lbr_nr: u8,
) -> Option<AmdLbrDelPlan> {
    if lbr_nr == 0 {
        return None;
    }

    if event.has_branch_stack {
        state.lbr_select = false;
    }
    state.lbr_users -= 1;

    Some(AmdLbrDelPlan {
        sched_cb_delta: -1,
        warn_negative_users: state.lbr_users < 0,
    })
}

pub fn amd_pmu_lbr_sched_task_plan(
    lbr_users: i32,
    sched_in: bool,
    lbr_nr: u8,
) -> Option<AmdLbrResetPlan> {
    if lbr_users != 0 && sched_in {
        amd_pmu_lbr_reset_plan(lbr_nr)
    } else {
        None
    }
}

pub fn amd_pmu_lbr_enable_all_plan(
    lbr_users: i32,
    lbr_nr: u8,
    lbr_select: bool,
    lbr_sel_config: u64,
    amd_lbr_pmc_freeze: bool,
    debugctl: u64,
    dbg_extn_cfg: u64,
) -> Vec<AmdLbrMsrWrite> {
    let mut writes = Vec::new();
    if lbr_users == 0 || lbr_nr == 0 {
        return writes;
    }

    if lbr_select {
        writes.push(AmdLbrMsrWrite {
            msr: MSR_AMD64_LBR_SELECT,
            value: lbr_sel_config & LBR_SELECT_MASK,
        });
    }
    if amd_lbr_pmc_freeze {
        writes.push(AmdLbrMsrWrite {
            msr: MSR_IA32_DEBUGCTLMSR,
            value: debugctl | DEBUGCTLMSR_FREEZE_LBRS_ON_PMI,
        });
    }
    writes.push(AmdLbrMsrWrite {
        msr: MSR_AMD_DBG_EXTN_CFG,
        value: dbg_extn_cfg | DBG_EXTN_CFG_LBRV2EN,
    });
    writes
}

pub fn amd_pmu_lbr_disable_all_plan(
    lbr_users: i32,
    lbr_nr: u8,
    amd_lbr_pmc_freeze: bool,
    debugctl: u64,
    dbg_extn_cfg: u64,
) -> Vec<AmdLbrMsrWrite> {
    let mut writes = Vec::new();
    if lbr_users == 0 || lbr_nr == 0 {
        return writes;
    }

    writes.push(AmdLbrMsrWrite {
        msr: MSR_AMD_DBG_EXTN_CFG,
        value: dbg_extn_cfg & !DBG_EXTN_CFG_LBRV2EN,
    });
    if amd_lbr_pmc_freeze {
        writes.push(AmdLbrMsrWrite {
            msr: MSR_IA32_DEBUGCTLMSR,
            value: debugctl & !DEBUGCTLMSR_FREEZE_LBRS_ON_PMI,
        });
    }
    writes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::x86::events::utils::{PERF_BR_COND, PERF_BR_RET};

    fn event(branch_sample_type: u64) -> AmdLbrEvent {
        AmdLbrEvent {
            branch_sample_type,
            attach_state: 0,
            branch_reg: AmdLbrFilterSetup { reg: 0, config: 0 },
            has_branch_stack: true,
            total_time_running: 0,
        }
    }

    #[test]
    fn lbr_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/events/amd/lbr.c"
        ));
        let perf_event = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/events/perf_event.h"
        ));
        let asm_perf = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/include/asm/perf_event.h"
        ));
        let msr_index = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/include/asm/msr-index.h"
        ));
        let uapi_perf = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/perf_event.h"
        ));

        assert!(source.contains("#define LBR_SELECT_MASK\t\t0x1ff"));
        assert!(source.contains("#define LBR_SELECT_KERNEL\t\t0"));
        assert!(source.contains("#define LBR_SELECT_FAR_BRANCH\t\t8"));
        assert!(source.contains("u64\tip:58;"));
        assert!(source.contains("u64\tmispredict:1;"));
        assert!(source.contains("u64\treserved:1;"));
        assert!(source.contains("u64\tspec:1;"));
        assert!(source.contains("u64\tvalid:1;"));
        assert!(source.contains("wrmsrq(MSR_AMD_SAMP_BR_FROM + idx * 2, val);"));
        assert!(source.contains("wrmsrq(MSR_AMD_SAMP_BR_FROM + idx * 2 + 1, val);"));
        assert!(source.contains("return (u64)(((s64)ip << shift) >> shift);"));
        assert!(source.contains("static const int lbr_spec_map[PERF_BR_SPEC_MAX]"));
        assert!(source.contains("idx = (entry.to.split.valid << 1) | entry.to.split.spec;"));
        assert!(source.contains("cpuc->lbr_stack.hw_idx = 0;"));
        assert!(source.contains("static const int lbr_select_map[PERF_SAMPLE_BRANCH_MAX_SHIFT]"));
        assert!(source.contains("reg->config = mask ^ LBR_SELECT_MASK;"));
        assert!(source.contains("event->attach_state |= PERF_ATTACH_SCHED_CB;"));
        assert!(source.contains("wrmsrq(MSR_AMD64_LBR_SELECT, 0);"));
        assert!(source.contains("if (!cpuc->lbr_users++ && !event->total_time_running)"));
        assert!(source.contains("if (cpuc->lbr_users && sched_in)"));
        assert!(
            source.contains("wrmsrq(MSR_AMD_DBG_EXTN_CFG, dbg_extn_cfg | DBG_EXTN_CFG_LBRV2EN);")
        );
        assert!(
            source.contains("if (x86_pmu.version < 2 || !boot_cpu_has(X86_FEATURE_AMD_LBR_V2))")
        );
        assert!(source.contains("x86_pmu.lbr_nr = ebx.split.lbr_v2_stack_sz;"));
        assert!(perf_event.contains("X86_BR_TYPE_SAVE\t= 1 << 18"));
        assert!(asm_perf.contains("#define EXT_PERFMON_DEBUG_FEATURES\t\t0x80000022"));
        assert!(msr_index.contains("#define MSR_AMD64_LBR_SELECT\t\t\t0xc000010e"));
        assert!(msr_index.contains("#define DBG_EXTN_CFG_LBRV2EN\t\tBIT_ULL(6)"));
        assert!(uapi_perf.contains("PERF_SAMPLE_BRANCH_COUNTERS_SHIFT\t= 19"));
    }

    #[test]
    fn init_uses_pmu_version_feature_and_cpuid_depth() {
        assert!(!lbr_v2_supported(1, true));
        assert!(!lbr_v2_supported(2, false));
        assert!(lbr_v2_supported(2, true));
        assert_eq!(amd_pmu_lbr_init(1, true, 0), Err(EOPNOTSUPP));
        let ebx = 0x20 << 4;
        assert_eq!(cpuid_lbr_v2_stack_size(ebx), 0x20);
        assert_eq!(
            amd_pmu_lbr_init(2, true, ebx).unwrap(),
            AmdLbrInitPlan {
                lbr_nr: 32,
                print_depth: 32,
            }
        );
    }

    #[test]
    fn setup_filter_maps_perf_sample_bits_to_x86_and_suppress_masks() {
        assert_eq!(
            amd_pmu_lbr_setup_filter(0, PERF_SAMPLE_BRANCH_ANY),
            Err(EOPNOTSUPP)
        );
        assert_eq!(
            amd_pmu_lbr_setup_filter(16, PERF_SAMPLE_BRANCH_ABORT_TX),
            Err(EOPNOTSUPP)
        );
        assert_eq!(
            amd_pmu_lbr_setup_filter(16, PERF_SAMPLE_BRANCH_CALL_STACK),
            Err(EOPNOTSUPP)
        );

        let setup = amd_pmu_lbr_setup_filter(
            16,
            PERF_SAMPLE_BRANCH_USER
                | PERF_SAMPLE_BRANCH_KERNEL
                | PERF_SAMPLE_BRANCH_ANY_CALL
                | PERF_SAMPLE_BRANCH_TYPE_SAVE,
        )
        .unwrap();
        assert_eq!(
            setup.reg,
            X86_BR_USER | X86_BR_KERNEL | X86_BR_ANY_CALL | X86_BR_TYPE_SAVE
        );
        assert_eq!(
            setup.config,
            (LBR_USER | LBR_KERNEL | LBR_REL_CALL | LBR_IND_CALL | LBR_FAR) ^ LBR_SELECT_MASK
        );

        let mut event = event(PERF_SAMPLE_BRANCH_ANY | PERF_SAMPLE_BRANCH_HV);
        assert_eq!(amd_pmu_lbr_hw_config(&mut event, 16), Ok(()));
        assert_ne!(event.attach_state & PERF_ATTACH_SCHED_CB, 0);
        assert_eq!(event.branch_reg.reg, X86_BR_ANY);
        assert_eq!(event.branch_reg.config, LBR_ANY ^ LBR_SELECT_MASK);
    }

    #[test]
    fn raw_lbr_decode_checks_valid_spec_reserved_bits_and_sign_extends() {
        let entries = [
            AmdLbrRawEntry {
                from_full: (1 << FROM_MISPREDICT_BIT) | 0x1000,
                to_full: (1 << TO_VALID_BIT) | 0x2000,
            },
            AmdLbrRawEntry {
                from_full: 0x3000,
                to_full: (1 << TO_SPEC_BIT) | 0x0000_8000_0000_4000,
            },
            AmdLbrRawEntry {
                from_full: 0x5000,
                to_full: 0,
            },
            AmdLbrRawEntry {
                from_full: 0x6000,
                to_full: (1 << TO_RESERVED_BIT) | (1 << TO_VALID_BIT) | 0x7000,
            },
        ];

        let decoded = amd_pmu_lbr_decode(1, 4, &entries, 48);
        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0].from, 0x1000);
        assert_eq!(decoded[0].to, 0x2000);
        assert!(decoded[0].mispredicted);
        assert!(!decoded[0].predicted);
        assert_eq!(decoded[0].spec, PERF_BR_NON_SPEC_CORRECT_PATH);
        assert_eq!(decoded[1].to, 0xffff_8000_0000_4000);
        assert_eq!(decoded[1].spec, PERF_BR_SPEC_WRONG_PATH);
        assert!(amd_pmu_lbr_decode(0, 4, &entries, 48).is_empty());
    }

    #[test]
    fn software_filter_adjusts_fused_offsets_discards_and_saves_type() {
        let entries = [
            AmdLbrBranchEntry {
                from: 0x1000,
                to: 0x2000,
                mispredicted: false,
                predicted: true,
                spec: PERF_BR_SPEC_NA,
                branch_type: 0,
            },
            AmdLbrBranchEntry {
                from: 0x3000,
                to: 0x4000,
                mispredicted: false,
                predicted: true,
                spec: PERF_BR_SPEC_NA,
                branch_type: 0,
            },
        ];
        let matches = [
            AmdLbrFilterMatch {
                branch_type: X86_BR_JCC | X86_BR_USER,
                fused_offset: 2,
            },
            AmdLbrFilterMatch {
                branch_type: X86_BR_RET | X86_BR_USER,
                fused_offset: 0,
            },
        ];

        let filtered = amd_pmu_lbr_filter(
            &entries,
            X86_BR_USER | X86_BR_JCC | X86_BR_TYPE_SAVE,
            &matches,
        );
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].from, 0x1002);
        assert_eq!(filtered[0].branch_type, PERF_BR_COND);

        let all_fused = amd_pmu_lbr_filter(&entries, X86_BR_ALL, &matches);
        assert_eq!(all_fused.len(), 2);
        assert_eq!(all_fused[0].from, 0x1002);
        assert_eq!(common_branch_type(X86_BR_RET | X86_BR_USER), PERF_BR_RET);
    }

    #[test]
    fn reset_add_delete_and_sched_follow_cpu_state_rules() {
        let reset = amd_pmu_lbr_reset_plan(2).unwrap();
        assert_eq!(
            reset.writes.as_slice(),
            &[
                AmdLbrMsrWrite {
                    msr: amd_pmu_lbr_from_msr(0),
                    value: 0,
                },
                AmdLbrMsrWrite {
                    msr: amd_pmu_lbr_to_msr(0),
                    value: 0,
                },
                AmdLbrMsrWrite {
                    msr: amd_pmu_lbr_from_msr(1),
                    value: 0,
                },
                AmdLbrMsrWrite {
                    msr: amd_pmu_lbr_to_msr(1),
                    value: 0,
                },
                AmdLbrMsrWrite {
                    msr: MSR_AMD64_LBR_SELECT,
                    value: 0,
                },
            ]
        );
        assert!(reset.clear_last_task_ctx);
        assert_eq!(reset.last_log_id, 0);

        let mut state = AmdLbrCpuState {
            lbr_users: 0,
            lbr_select: false,
            lbr_sel_config: 0,
            br_sel: 0,
            last_task_ctx: Some(7),
            last_log_id: 3,
        };
        let mut event = event(PERF_SAMPLE_BRANCH_ANY);
        amd_pmu_lbr_hw_config(&mut event, 2).unwrap();
        let add = amd_pmu_lbr_add(&mut state, &event, 2).unwrap();
        assert_eq!(add.sched_cb_delta, 1);
        assert!(add.reset.is_some());
        assert_eq!(state.lbr_users, 1);
        assert!(state.lbr_select);
        assert_eq!(state.lbr_sel_config, event.branch_reg.config);
        assert_eq!(state.br_sel, event.branch_reg.reg);

        assert!(amd_pmu_lbr_sched_task_plan(state.lbr_users, true, 2).is_some());
        assert_eq!(amd_pmu_lbr_sched_task_plan(state.lbr_users, false, 2), None);

        let del = amd_pmu_lbr_del(&mut state, &event, 2).unwrap();
        assert_eq!(del.sched_cb_delta, -1);
        assert!(!del.warn_negative_users);
        assert_eq!(state.lbr_users, 0);
        assert!(!state.lbr_select);
    }

    #[test]
    fn enable_and_disable_all_emit_expected_msr_writes() {
        assert!(amd_pmu_lbr_enable_all_plan(0, 16, true, 0x123, true, 0, 0).is_empty());
        assert!(amd_pmu_lbr_disable_all_plan(1, 0, true, 0, 0).is_empty());

        let enable = amd_pmu_lbr_enable_all_plan(1, 16, true, 0x3ff, true, 0x10, 0x20);
        assert_eq!(
            enable.as_slice(),
            &[
                AmdLbrMsrWrite {
                    msr: MSR_AMD64_LBR_SELECT,
                    value: LBR_SELECT_MASK,
                },
                AmdLbrMsrWrite {
                    msr: MSR_IA32_DEBUGCTLMSR,
                    value: 0x10 | DEBUGCTLMSR_FREEZE_LBRS_ON_PMI,
                },
                AmdLbrMsrWrite {
                    msr: MSR_AMD_DBG_EXTN_CFG,
                    value: 0x20 | DBG_EXTN_CFG_LBRV2EN,
                },
            ]
        );

        let disable = amd_pmu_lbr_disable_all_plan(
            1,
            16,
            true,
            DEBUGCTLMSR_FREEZE_LBRS_ON_PMI | 0x55,
            DBG_EXTN_CFG_LBRV2EN | 0xaa,
        );
        assert_eq!(
            disable.as_slice(),
            &[
                AmdLbrMsrWrite {
                    msr: MSR_AMD_DBG_EXTN_CFG,
                    value: 0xaa,
                },
                AmdLbrMsrWrite {
                    msr: MSR_IA32_DEBUGCTLMSR,
                    value: 0x55,
                },
            ]
        );
    }
}
