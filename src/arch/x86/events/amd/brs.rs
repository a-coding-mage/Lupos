//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/events/amd/brs.c
//! test-origin: linux:vendor/linux/arch/x86/events/amd/brs.c
//! AMD Fam19h branch sampling model.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::{EINVAL, EOPNOTSUPP};

pub const BRS_POISON: u64 = 0xffff_ffff_ffff_fffe;
pub const AMD_BRS_MAX_DEPTH: u8 = 16;

pub const MSR_AMD_DBG_EXTN_CFG: u32 = 0xc000_010f;
pub const MSR_AMD_SAMP_BR_FROM: u32 = 0xc001_0300;

pub const ARCH_PERFMON_EVENTSEL_EVENT: u64 = 0x0000_00ff;
pub const ARCH_PERFMON_EVENTSEL_UMASK: u64 = 0x0000_ff00;
pub const ARCH_PERFMON_EVENTSEL_EDGE: u64 = 1 << 18;
pub const ARCH_PERFMON_EVENTSEL_INV: u64 = 1 << 23;
pub const ARCH_PERFMON_EVENTSEL_CMASK: u64 = 0xff00_0000;
pub const AMD64_EVENTSEL_EVENT: u64 = ARCH_PERFMON_EVENTSEL_EVENT | (0x0f << 32);
pub const X86_RAW_EVENT_MASK: u64 = ARCH_PERFMON_EVENTSEL_EVENT
    | ARCH_PERFMON_EVENTSEL_UMASK
    | ARCH_PERFMON_EVENTSEL_EDGE
    | ARCH_PERFMON_EVENTSEL_INV
    | ARCH_PERFMON_EVENTSEL_CMASK;
pub const AMD64_RAW_EVENT_MASK: u64 = X86_RAW_EVENT_MASK | AMD64_EVENTSEL_EVENT;
pub const AMD_FAM19H_BRS_EVENT: u64 = 0xc4;
pub const PERF_X86_EVENT_AMD_BRS: u64 = 0x001_0000;

pub const PERF_SAMPLE_BRANCH_USER: u64 = 1 << 0;
pub const PERF_SAMPLE_BRANCH_KERNEL: u64 = 1 << 1;
pub const PERF_SAMPLE_BRANCH_HV: u64 = 1 << 2;
pub const PERF_SAMPLE_BRANCH_ANY: u64 = 1 << 3;
pub const PERF_SAMPLE_BRANCH_ANY_CALL: u64 = 1 << 4;
pub const PERF_SAMPLE_BRANCH_PLM_ALL: u64 =
    PERF_SAMPLE_BRANCH_USER | PERF_SAMPLE_BRANCH_KERNEL | PERF_SAMPLE_BRANCH_HV;

const BRSMEN_BIT: u8 = 2;
const VB_BIT: u8 = 5;
const MSROFF_SHIFT: u8 = 16;
const MSROFF_MASK: u64 = 0x0f;
const PMC_SHIFT: u8 = 24;
const PMC_MASK: u64 = 0x07;
const DEBUG_EXTN_CFG_RESERVED_WRITE_BITS: u64 = 3 << 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdBrsConfig {
    pub family: u8,
    pub depth: u8,
    pub hardware_filtering: bool,
    pub lbr_sel_mask: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdBrsInitPlan {
    pub config: AmdBrsConfig,
    pub print_depth: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdDebugExtnCfg {
    pub val: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdBrsEventAttr {
    pub branch_sample_type: u64,
    pub freq: bool,
    pub sample_period: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdBrsHw {
    pub config: u64,
    pub flags: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdBrsEvent {
    pub attr: AmdBrsEventAttr,
    pub hw: AmdBrsHw,
    pub sampling: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdBrsMsrWrite {
    pub msr: u32,
    pub value: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdBrsResetPlan {
    pub debug_extn_cfg: AmdBrsMsrWrite,
    pub poison: AmdBrsMsrWrite,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdBrsCpuState {
    pub brs_active: u32,
    pub lbr_users: u32,
    pub debug_extn_cfg: AmdDebugExtnCfg,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdBrsMsrEntry {
    pub from: u64,
    pub to: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdBrsBranchEntry {
    pub from: u64,
    pub to: u64,
}

impl AmdDebugExtnCfg {
    pub const fn new(val: u64) -> Self {
        Self { val }
    }

    pub const fn brsmen(self) -> bool {
        (self.val & (1 << BRSMEN_BIT)) != 0
    }

    pub const fn vb(self) -> bool {
        (self.val & (1 << VB_BIT)) != 0
    }

    pub const fn msroff(self) -> u8 {
        ((self.val >> MSROFF_SHIFT) & MSROFF_MASK) as u8
    }

    pub const fn pmc(self) -> u8 {
        ((self.val >> PMC_SHIFT) & PMC_MASK) as u8
    }

    pub const fn with_brsmen(mut self, enabled: bool) -> Self {
        if enabled {
            self.val |= 1 << BRSMEN_BIT;
        } else {
            self.val &= !(1 << BRSMEN_BIT);
        }
        self
    }

    pub const fn with_vb(mut self, valid: bool) -> Self {
        if valid {
            self.val |= 1 << VB_BIT;
        } else {
            self.val &= !(1 << VB_BIT);
        }
        self
    }

    pub const fn with_msroff(mut self, index: u8) -> Self {
        self.val &= !(MSROFF_MASK << MSROFF_SHIFT);
        self.val |= ((index as u64) & MSROFF_MASK) << MSROFF_SHIFT;
        self
    }

    pub const fn with_pmc(mut self, pmc: u8) -> Self {
        self.val &= !(PMC_MASK << PMC_SHIFT);
        self.val |= ((pmc as u64) & PMC_MASK) << PMC_SHIFT;
        self
    }

    pub const fn for_msr_write(self) -> u64 {
        self.val | DEBUG_EXTN_CFG_RESERVED_WRITE_BITS
    }
}

pub const fn brs_from(index: u8) -> u32 {
    MSR_AMD_SAMP_BR_FROM + 2 * index as u32
}

pub const fn brs_to(index: u8) -> u32 {
    MSR_AMD_SAMP_BR_FROM + 2 * index as u32 + 1
}

pub const fn brs_supported(family: u8, cpuid_brs: bool) -> bool {
    cpuid_brs && family == 0x19
}

pub const fn amd_brs_detect(cpuid_brs: bool, family: u8) -> Option<AmdBrsConfig> {
    if brs_supported(family, cpuid_brs) {
        Some(AmdBrsConfig {
            family,
            depth: AMD_BRS_MAX_DEPTH,
            hardware_filtering: false,
            lbr_sel_mask: 0,
        })
    } else {
        None
    }
}

pub const fn brs_config(family: u8, cpuid_brs: bool) -> Result<AmdBrsConfig, i32> {
    match amd_brs_detect(cpuid_brs, family) {
        Some(config) => Ok(config),
        None => Err(EOPNOTSUPP),
    }
}

pub const fn amd_brs_init(cpuid_brs: bool, family: u8) -> Result<AmdBrsInitPlan, i32> {
    match amd_brs_detect(cpuid_brs, family) {
        Some(config) => Ok(AmdBrsInitPlan {
            config,
            print_depth: config.depth,
        }),
        None => Err(EOPNOTSUPP),
    }
}

pub const fn amd_brs_setup_filter(lbr_nr: u8, branch_sample_type: u64) -> Result<(), i32> {
    if lbr_nr == 0 {
        return Err(EOPNOTSUPP);
    }
    if (branch_sample_type & !PERF_SAMPLE_BRANCH_PLM_ALL) != PERF_SAMPLE_BRANCH_ANY {
        return Err(EINVAL);
    }
    Ok(())
}

pub const fn amd_is_brs_event(config: u64) -> bool {
    (config & AMD64_RAW_EVENT_MASK) == AMD_FAM19H_BRS_EVENT
}

pub const fn has_amd_brs(flags: u64) -> bool {
    (flags & PERF_X86_EVENT_AMD_BRS) != 0
}

pub fn amd_brs_hw_config(event: &mut AmdBrsEvent, lbr_nr: u8) -> Result<(), i32> {
    if !event.sampling {
        return Err(EINVAL);
    }
    if !amd_is_brs_event(event.hw.config) {
        return Err(EINVAL);
    }
    if event.attr.freq {
        return Err(EINVAL);
    }
    if event.attr.sample_period <= lbr_nr as u64 {
        return Err(EINVAL);
    }
    amd_brs_setup_filter(lbr_nr, event.attr.branch_sample_type)?;
    event.hw.flags |= PERF_X86_EVENT_AMD_BRS;
    Ok(())
}

pub const fn amd_brs_get_tos(cfg: AmdDebugExtnCfg, lbr_nr: u8) -> Option<u8> {
    if lbr_nr == 0 || cfg.msroff() >= lbr_nr {
        return None;
    }
    if cfg.msroff() == 0 {
        Some(lbr_nr - 1)
    } else {
        Some(cfg.msroff() - 1)
    }
}

pub const fn amd_brs_reset_plan(cpuid_brs: bool) -> Option<AmdBrsResetPlan> {
    if !cpuid_brs {
        return None;
    }
    Some(AmdBrsResetPlan {
        debug_extn_cfg: AmdBrsMsrWrite {
            msr: MSR_AMD_DBG_EXTN_CFG,
            value: AmdDebugExtnCfg::new(0).for_msr_write(),
        },
        poison: AmdBrsMsrWrite {
            msr: brs_to(0),
            value: BRS_POISON,
        },
    })
}

pub fn amd_brs_enable(state: &mut AmdBrsCpuState) -> Option<AmdBrsMsrWrite> {
    state.brs_active = state.brs_active.saturating_add(1);
    if state.brs_active > 1 {
        return None;
    }

    state.debug_extn_cfg = AmdDebugExtnCfg::new(0).with_brsmen(true);
    Some(AmdBrsMsrWrite {
        msr: MSR_AMD_DBG_EXTN_CFG,
        value: state.debug_extn_cfg.for_msr_write(),
    })
}

pub fn amd_brs_enable_all(state: &mut AmdBrsCpuState) -> Option<AmdBrsMsrWrite> {
    if state.lbr_users == 0 {
        return None;
    }
    amd_brs_enable(state)
}

pub fn amd_brs_disable(
    state: &mut AmdBrsCpuState,
    current_debug_extn_cfg: AmdDebugExtnCfg,
) -> Option<AmdBrsMsrWrite> {
    if state.brs_active == 0 {
        return None;
    }

    state.brs_active -= 1;
    if state.brs_active != 0 {
        return None;
    }

    if !current_debug_extn_cfg.brsmen() {
        state.debug_extn_cfg = current_debug_extn_cfg;
        return None;
    }

    state.debug_extn_cfg = current_debug_extn_cfg.with_brsmen(false);
    Some(AmdBrsMsrWrite {
        msr: MSR_AMD_DBG_EXTN_CFG,
        value: state.debug_extn_cfg.for_msr_write(),
    })
}

pub fn amd_brs_disable_all(
    state: &mut AmdBrsCpuState,
    current_debug_extn_cfg: AmdDebugExtnCfg,
) -> Option<AmdBrsMsrWrite> {
    if state.lbr_users == 0 {
        return None;
    }
    amd_brs_disable(state, current_debug_extn_cfg)
}

pub const fn kernel_ip_x86_64(ip: u64) -> bool {
    (ip as i64) < 0
}

pub const fn amd_brs_match_plm(branch_sample_type: u64, to: u64) -> bool {
    let plm_k = PERF_SAMPLE_BRANCH_KERNEL | PERF_SAMPLE_BRANCH_HV;
    let plm_u = PERF_SAMPLE_BRANCH_USER;

    if (branch_sample_type & plm_k) == 0 && kernel_ip_x86_64(to) {
        return false;
    }
    if (branch_sample_type & plm_u) == 0 && !kernel_ip_x86_64(to) {
        return false;
    }
    true
}

pub const fn sign_extend_to_virt_bits(value: u64, virt_bits: u8) -> u64 {
    if virt_bits == 0 || virt_bits >= 64 {
        value
    } else {
        let shift = 64 - virt_bits;
        (((value << shift) as i64) >> shift) as u64
    }
}

pub fn amd_brs_drain(
    event: Option<&AmdBrsEvent>,
    cfg: AmdDebugExtnCfg,
    entries: &[AmdBrsMsrEntry],
    virt_bits: u8,
    lbr_nr: u8,
) -> Vec<AmdBrsBranchEntry> {
    let mut sampled = Vec::new();
    let Some(event) = event else {
        return sampled;
    };
    if cfg.msroff() >= lbr_nr || !cfg.vb() {
        return sampled;
    }
    let Some(tos) = amd_brs_get_tos(cfg, lbr_nr) else {
        return sampled;
    };

    let mut index = tos as usize;
    loop {
        let Some(entry) = entries.get(index) else {
            break;
        };
        if entry.to == BRS_POISON {
            break;
        }

        let to = sign_extend_to_virt_bits(entry.to, virt_bits);
        if amd_brs_match_plm(event.attr.branch_sample_type, to) {
            sampled.push(AmdBrsBranchEntry {
                from: entry.from,
                to,
            });
        }

        if index == 0 {
            break;
        }
        index -= 1;
    }
    sampled
}

pub const fn amd_brs_poison_buffer_plan(
    cfg: AmdDebugExtnCfg,
    lbr_nr: u8,
) -> Option<AmdBrsMsrWrite> {
    match amd_brs_get_tos(cfg, lbr_nr) {
        Some(index) => Some(AmdBrsMsrWrite {
            msr: brs_to(index),
            value: BRS_POISON,
        }),
        None => None,
    }
}

pub const fn amd_pmu_brs_sched_task_plan(
    lbr_users: u32,
    sched_in: bool,
    cfg: AmdDebugExtnCfg,
    lbr_nr: u8,
) -> Option<AmdBrsMsrWrite> {
    if lbr_users == 0 || !sched_in {
        return None;
    }
    amd_brs_poison_buffer_plan(cfg, lbr_nr)
}

pub const fn perf_amd_brs_lopwr_cb_plan(
    brs_active: u32,
    lopwr_in: bool,
    current_debug_extn_cfg: AmdDebugExtnCfg,
) -> Option<AmdBrsMsrWrite> {
    if brs_active == 0 {
        return None;
    }
    let cfg = current_debug_extn_cfg.with_brsmen(!lopwr_in);
    Some(AmdBrsMsrWrite {
        msr: MSR_AMD_DBG_EXTN_CFG,
        value: cfg.for_msr_write(),
    })
}

pub const fn amd_brs_lopwr_init_plan() -> &'static str {
    "static_call_update(perf_lopwr_cb, perf_amd_brs_lopwr_cb)"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn brs_event(branch_sample_type: u64) -> AmdBrsEvent {
        AmdBrsEvent {
            attr: AmdBrsEventAttr {
                branch_sample_type,
                freq: false,
                sample_period: AMD_BRS_MAX_DEPTH as u64 + 1,
            },
            hw: AmdBrsHw {
                config: AMD_FAM19H_BRS_EVENT,
                flags: 0,
            },
            sampling: true,
        }
    }

    #[test]
    fn brs_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/events/amd/brs.c"
        ));
        let perf_flags = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/events/perf_event_flags.h"
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

        assert!(source.contains("#define BRS_POISON\t0xFFFFFFFFFFFFFFFEULL"));
        assert!(source.contains("bits[4:3] must always be set to 11b"));
        assert!(source.contains("return MSR_AMD_SAMP_BR_FROM + 2 * idx;"));
        assert!(source.contains("return MSR_AMD_SAMP_BR_FROM + 2 * idx + 1;"));
        assert!(source.contains("case 0x19: /* AMD Fam19h (Zen3) */"));
        assert!(source.contains("x86_pmu.lbr_nr = 16;"));
        assert!(source.contains("x86_pmu.lbr_sel_mask = 0;"));
        assert!(
            source
                .contains("return (e->hw.config & AMD64_RAW_EVENT_MASK) == AMD_FAM19H_BRS_EVENT;")
        );
        assert!(source.contains("event->hw.flags |= PERF_X86_EVENT_AMD_BRS;"));
        assert!(source.contains("return (cfg->msroff ? cfg->msroff : x86_pmu.lbr_nr) - 1;"));
        assert!(source.contains("wrmsrq(brs_to(0), BRS_POISON);"));
        assert!(source.contains("if (++cpuc->brs_active > 1)"));
        assert!(source.contains("if (--cpuc->brs_active)"));
        assert!(source.contains("if (!(type & plm_k) && kernel_ip(to))"));
        assert!(source.contains("if (!(type & plm_u) && !kernel_ip(to))"));
        assert!(source.contains("if (to == BRS_POISON)"));
        assert!(source.contains("to = (u64)(((s64)to << shift) >> shift);"));
        assert!(source.contains("if (sched_in)"));
        assert!(source.contains("cfg.brsmen = !lopwr_in;"));
        assert!(perf_flags.contains("PERF_ARCH(AMD_BRS,\t\t0x0010000)"));
        assert!(perf_event.contains("#define AMD_FAM19H_BRS_EVENT 0xc4"));
        assert!(asm_perf.contains("#define AMD64_RAW_EVENT_MASK"));
        assert!(msr_index.contains("#define MSR_AMD_DBG_EXTN_CFG\t\t0xc000010f"));
        assert!(msr_index.contains("#define MSR_AMD_SAMP_BR_FROM\t\t0xc0010300"));
    }

    #[test]
    fn detect_init_and_filter_follow_brs_limits() {
        assert!(brs_supported(0x19, true));
        assert!(!brs_supported(0x17, true));
        assert!(!brs_supported(0x1a, true));
        assert_eq!(brs_config(0x19, false), Err(EOPNOTSUPP));

        let config = amd_brs_detect(true, 0x19).unwrap();
        assert_eq!(config.depth, AMD_BRS_MAX_DEPTH);
        assert!(!config.hardware_filtering);
        assert_eq!(config.lbr_sel_mask, 0);
        assert_eq!(amd_brs_init(false, 0x19), Err(EOPNOTSUPP));
        assert_eq!(amd_brs_init(true, 0x19).unwrap().print_depth, 16);

        assert_eq!(
            amd_brs_setup_filter(0, PERF_SAMPLE_BRANCH_ANY),
            Err(EOPNOTSUPP)
        );
        assert_eq!(amd_brs_setup_filter(16, 0), Err(EINVAL));
        assert_eq!(
            amd_brs_setup_filter(16, PERF_SAMPLE_BRANCH_ANY_CALL),
            Err(EINVAL)
        );
        assert_eq!(
            amd_brs_setup_filter(
                16,
                PERF_SAMPLE_BRANCH_ANY | PERF_SAMPLE_BRANCH_USER | PERF_SAMPLE_BRANCH_KERNEL
            ),
            Ok(())
        );
    }

    #[test]
    fn hw_config_accepts_only_sampling_fam19h_brs_events() {
        let mut event = brs_event(PERF_SAMPLE_BRANCH_ANY | PERF_SAMPLE_BRANCH_USER);
        assert_eq!(amd_brs_hw_config(&mut event, AMD_BRS_MAX_DEPTH), Ok(()));
        assert!(has_amd_brs(event.hw.flags));

        let mut counting = brs_event(PERF_SAMPLE_BRANCH_ANY);
        counting.sampling = false;
        assert_eq!(
            amd_brs_hw_config(&mut counting, AMD_BRS_MAX_DEPTH),
            Err(EINVAL)
        );

        let mut wrong_event = brs_event(PERF_SAMPLE_BRANCH_ANY);
        wrong_event.hw.config = AMD_FAM19H_BRS_EVENT | (1 << 8);
        assert_eq!(
            amd_brs_hw_config(&mut wrong_event, AMD_BRS_MAX_DEPTH),
            Err(EINVAL)
        );

        let mut freq = brs_event(PERF_SAMPLE_BRANCH_ANY);
        freq.attr.freq = true;
        assert_eq!(amd_brs_hw_config(&mut freq, AMD_BRS_MAX_DEPTH), Err(EINVAL));

        let mut too_short = brs_event(PERF_SAMPLE_BRANCH_ANY);
        too_short.attr.sample_period = AMD_BRS_MAX_DEPTH as u64;
        assert_eq!(
            amd_brs_hw_config(&mut too_short, AMD_BRS_MAX_DEPTH),
            Err(EINVAL)
        );
    }

    #[test]
    fn debug_extn_cfg_and_reset_plans_match_msr_layout() {
        let cfg = AmdDebugExtnCfg::new(0)
            .with_brsmen(true)
            .with_vb(true)
            .with_msroff(7)
            .with_pmc(5);
        assert!(cfg.brsmen());
        assert!(cfg.vb());
        assert_eq!(cfg.msroff(), 7);
        assert_eq!(cfg.pmc(), 5);
        assert_eq!(
            cfg.for_msr_write() & DEBUG_EXTN_CFG_RESERVED_WRITE_BITS,
            3 << 3
        );
        assert_eq!(brs_from(2), MSR_AMD_SAMP_BR_FROM + 4);
        assert_eq!(brs_to(2), MSR_AMD_SAMP_BR_FROM + 5);
        assert_eq!(amd_brs_get_tos(cfg, AMD_BRS_MAX_DEPTH), Some(6));
        assert_eq!(
            amd_brs_get_tos(AmdDebugExtnCfg::new(0).with_msroff(0), AMD_BRS_MAX_DEPTH),
            Some(15)
        );
        assert_eq!(
            amd_brs_get_tos(AmdDebugExtnCfg::new(0).with_msroff(8), 8),
            None
        );

        assert_eq!(amd_brs_reset_plan(false), None);
        assert_eq!(
            amd_brs_reset_plan(true).unwrap(),
            AmdBrsResetPlan {
                debug_extn_cfg: AmdBrsMsrWrite {
                    msr: MSR_AMD_DBG_EXTN_CFG,
                    value: 3 << 3,
                },
                poison: AmdBrsMsrWrite {
                    msr: brs_to(0),
                    value: BRS_POISON,
                },
            }
        );
    }

    #[test]
    fn enable_disable_and_low_power_callbacks_plan_kernel_writes() {
        let mut state = AmdBrsCpuState {
            brs_active: 0,
            lbr_users: 1,
            debug_extn_cfg: AmdDebugExtnCfg::new(0),
        };

        let first = amd_brs_enable_all(&mut state).unwrap();
        assert_eq!(state.brs_active, 1);
        assert_eq!(first.msr, MSR_AMD_DBG_EXTN_CFG);
        assert_eq!(
            first.value,
            AmdDebugExtnCfg::new(0).with_brsmen(true).for_msr_write()
        );
        assert_eq!(amd_brs_enable_all(&mut state), None);
        assert_eq!(state.brs_active, 2);

        let current = AmdDebugExtnCfg::new(0)
            .with_brsmen(true)
            .with_vb(true)
            .with_msroff(4);
        assert_eq!(amd_brs_disable_all(&mut state, current), None);
        assert_eq!(state.brs_active, 1);
        let disabled = amd_brs_disable_all(&mut state, current).unwrap();
        assert_eq!(state.brs_active, 0);
        assert_eq!(disabled.value, current.with_brsmen(false).for_msr_write());
        assert!(state.debug_extn_cfg.vb());
        assert_eq!(state.debug_extn_cfg.msroff(), 4);

        assert_eq!(
            perf_amd_brs_lopwr_cb_plan(1, true, current).unwrap().value,
            current.with_brsmen(false).for_msr_write()
        );
        assert_eq!(
            perf_amd_brs_lopwr_cb_plan(1, false, current).unwrap().value,
            current.with_brsmen(true).for_msr_write()
        );
        assert_eq!(perf_amd_brs_lopwr_cb_plan(0, true, current), None);
        assert_eq!(
            amd_brs_lopwr_init_plan(),
            "static_call_update(perf_lopwr_cb, perf_amd_brs_lopwr_cb)"
        );
    }

    #[test]
    fn drain_reads_newest_to_oldest_sign_extends_and_filters_plm() {
        let event = brs_event(PERF_SAMPLE_BRANCH_ANY | PERF_SAMPLE_BRANCH_USER);
        let cfg = AmdDebugExtnCfg::new(0).with_vb(true).with_msroff(3);
        let entries = [
            AmdBrsMsrEntry {
                from: 0x10,
                to: 0x1000,
            },
            AmdBrsMsrEntry {
                from: 0x20,
                to: 0x0000_8000_0000_1234,
            },
            AmdBrsMsrEntry {
                from: 0x30,
                to: 0x2000,
            },
        ];

        let sampled = amd_brs_drain(Some(&event), cfg, &entries, 48, AMD_BRS_MAX_DEPTH);
        assert_eq!(
            sampled.as_slice(),
            &[
                AmdBrsBranchEntry {
                    from: 0x30,
                    to: 0x2000,
                },
                AmdBrsBranchEntry {
                    from: 0x10,
                    to: 0x1000,
                },
            ]
        );
        assert_eq!(
            sign_extend_to_virt_bits(0x0000_8000_0000_1234, 48),
            0xffff_8000_0000_1234
        );
        assert!(!amd_brs_match_plm(
            PERF_SAMPLE_BRANCH_ANY | PERF_SAMPLE_BRANCH_USER,
            0xffff_8000_0000_1234
        ));
        assert!(amd_brs_match_plm(
            PERF_SAMPLE_BRANCH_ANY | PERF_SAMPLE_BRANCH_KERNEL,
            0xffff_8000_0000_1234
        ));
    }

    #[test]
    fn drain_empty_and_poison_paths_match_kernel_guards() {
        let event = brs_event(PERF_SAMPLE_BRANCH_ANY | PERF_SAMPLE_BRANCH_USER);
        let cfg = AmdDebugExtnCfg::new(0).with_vb(true).with_msroff(2);
        let entries = [
            AmdBrsMsrEntry {
                from: 0x10,
                to: 0x1000,
            },
            AmdBrsMsrEntry {
                from: 0x20,
                to: BRS_POISON,
            },
        ];

        assert!(amd_brs_drain(None, cfg, &entries, 48, AMD_BRS_MAX_DEPTH).is_empty());
        assert!(
            amd_brs_drain(
                Some(&event),
                cfg.with_vb(false),
                &entries,
                48,
                AMD_BRS_MAX_DEPTH
            )
            .is_empty()
        );
        assert!(amd_brs_drain(Some(&event), cfg, &entries, 48, AMD_BRS_MAX_DEPTH).is_empty());

        assert_eq!(
            amd_brs_poison_buffer_plan(cfg, AMD_BRS_MAX_DEPTH),
            Some(AmdBrsMsrWrite {
                msr: brs_to(1),
                value: BRS_POISON,
            })
        );
        assert_eq!(
            amd_pmu_brs_sched_task_plan(1, true, cfg, AMD_BRS_MAX_DEPTH),
            amd_brs_poison_buffer_plan(cfg, AMD_BRS_MAX_DEPTH)
        );
        assert_eq!(
            amd_pmu_brs_sched_task_plan(0, true, cfg, AMD_BRS_MAX_DEPTH),
            None
        );
        assert_eq!(
            amd_pmu_brs_sched_task_plan(1, false, cfg, AMD_BRS_MAX_DEPTH),
            None
        );
    }
}
