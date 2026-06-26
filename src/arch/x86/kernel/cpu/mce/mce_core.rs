//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/mce
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/mce
//! Core x86 Machine Check Architecture state.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/cpu/mce/core.c

use super::{amd, intel};
use crate::arch::x86::kernel::cpu::{CpuFeatures, CpuVendor};
use crate::include::uapi::errno::{EINVAL, ENODEV};

pub const MAX_NR_BANKS: usize = 64;
pub const PAGE_SHIFT: u8 = 12;

pub const MCG_BANKCNT_MASK: u64 = 0xff;
pub const MCG_CTL_P: u64 = 1 << 8;
pub const MCG_EXT_P: u64 = 1 << 9;
pub const MCG_CMCI_P: u64 = 1 << 10;
pub const MCG_SEAM_NR: u64 = 1 << 12;
pub const MCG_EXT_CNT_MASK: u64 = 0xff0000;
pub const MCG_EXT_CNT_SHIFT: u8 = 16;
pub const MCG_SER_P: u64 = 1 << 24;
pub const MCG_ELOG_P: u64 = 1 << 26;
pub const MCG_LMCE_P: u64 = 1 << 27;

pub const MCG_STATUS_RIPV: u64 = 1 << 0;
pub const MCG_STATUS_EIPV: u64 = 1 << 1;
pub const MCG_STATUS_MCIP: u64 = 1 << 2;
pub const MCG_STATUS_LMCES: u64 = 1 << 3;
pub const MCG_STATUS_SEAM_NR: u64 = 1 << 12;

pub const MCG_EXT_CTL_LMCE_EN: u64 = 1;

pub const MCI_STATUS_VAL: u64 = 1 << 63;
pub const MCI_STATUS_OVER: u64 = 1 << 62;
pub const MCI_STATUS_UC: u64 = 1 << 61;
pub const MCI_STATUS_EN: u64 = 1 << 60;
pub const MCI_STATUS_MISCV: u64 = 1 << 59;
pub const MCI_STATUS_ADDRV: u64 = 1 << 58;
pub const MCI_STATUS_PCC: u64 = 1 << 57;
pub const MCI_STATUS_S: u64 = 1 << 56;
pub const MCI_STATUS_AR: u64 = 1 << 55;
pub const MCI_STATUS_CEC_SHIFT: u8 = 38;
pub const MCI_STATUS_CEC_MASK: u64 = ((1u64 << 15) - 1) << MCI_STATUS_CEC_SHIFT;
pub const MCI_STATUS_TCC: u64 = 1 << 55;
pub const MCI_STATUS_PADDRV: u64 = 1 << 54;
pub const MCI_STATUS_SYNDV: u64 = 1 << 53;
pub const MCI_STATUS_DEFERRED: u64 = 1 << 44;
pub const MCI_STATUS_POISON: u64 = 1 << 43;
pub const MCI_STATUS_SCRUB: u64 = 1 << 40;

pub const MCACOD: u64 = 0xefff;
pub const MCACOD_SCRUB: u64 = 0x00c0;
pub const MCACOD_SCRUBMSK: u64 = 0xeff0;
pub const MCACOD_L3WB: u64 = 0x017a;
pub const MCACOD_DATA: u64 = 0x0134;
pub const MCACOD_INSTR: u64 = 0x0150;

pub const MCI_UC_S: u64 = MCI_STATUS_UC | MCI_STATUS_S;
pub const MCI_UC_AR: u64 = MCI_STATUS_UC | MCI_STATUS_AR;
pub const MCI_UC_SAR: u64 = MCI_STATUS_UC | MCI_STATUS_S | MCI_STATUS_AR;
pub const MCI_ADDR: u64 = MCI_STATUS_ADDRV | MCI_STATUS_MISCV;

pub const MCI_CTL2_CMCI_EN: u64 = 1 << 30;
pub const MCI_CTL2_CMCI_THRESHOLD_MASK: u64 = 0x7fff;

pub const MCE_HANDLED_CEC: u64 = 1 << 0;
pub const MCE_HANDLED_UC: u64 = 1 << 1;
pub const MCE_HANDLED_EXTLOG: u64 = 1 << 2;
pub const MCE_HANDLED_NFIT: u64 = 1 << 3;
pub const MCE_HANDLED_EDAC: u64 = 1 << 4;
pub const MCE_HANDLED_MCELOG: u64 = 1 << 5;
pub const MCE_IN_KERNEL_RECOV: u64 = 1 << 6;
pub const MCE_IN_KERNEL_COPYIN: u64 = 1 << 7;
pub const MCE_CHECK_DFR_REGS: u64 = 1 << 8;

pub const MCE_LOG_MIN_LEN: usize = 32;
pub const MCE_LOG_SIGNATURE: &[u8; 12] = b"MACHINECHECK";

pub const MSR_IA32_MCG_CAP: u32 = 0x0000_0179;
pub const MSR_IA32_MCG_STATUS: u32 = 0x0000_017a;
pub const MSR_IA32_MCG_CTL: u32 = 0x0000_017b;
pub const MSR_ERROR_CONTROL: u32 = 0x0000_017f;
pub const MSR_IA32_MCG_EXT_CTL: u32 = 0x0000_04d0;
pub const MSR_IA32_FEAT_CTL: u32 = 0x0000_003a;
pub const MSR_IA32_MC0_CTL: u32 = 0x0000_0400;
pub const MSR_IA32_MC0_STATUS: u32 = 0x0000_0401;
pub const MSR_IA32_MC0_ADDR: u32 = 0x0000_0402;
pub const MSR_IA32_MC0_MISC: u32 = 0x0000_0403;
pub const MSR_IA32_MC0_CTL2: u32 = 0x0000_0280;

pub const MSR_AMD64_SMCA_MC0_CTL: u32 = 0xc000_2000;
pub const MSR_AMD64_SMCA_MC0_STATUS: u32 = 0xc000_2001;
pub const MSR_AMD64_SMCA_MC0_ADDR: u32 = 0xc000_2002;
pub const MSR_AMD64_SMCA_MC0_MISC0: u32 = 0xc000_2003;
pub const MSR_AMD64_SMCA_MC0_CONFIG: u32 = 0xc000_2004;
pub const MSR_AMD64_SMCA_MC0_IPID: u32 = 0xc000_2005;
pub const MSR_AMD64_SMCA_MC0_SYND: u32 = 0xc000_2006;
pub const MSR_AMD64_SMCA_MC0_DESTAT: u32 = 0xc000_2008;
pub const MSR_AMD64_SMCA_MC0_DEADDR: u32 = 0xc000_2009;
pub const MSR_AMD64_SMCA_MC0_MISC1: u32 = 0xc000_200a;
pub const MSR_AMD64_SMCA_MC0_SYND1: u32 = 0xc000_200e;
pub const MSR_AMD64_SMCA_MC0_SYND2: u32 = 0xc000_200f;

pub const MCA_CTL: u8 = 0;
pub const MCA_STATUS: u8 = 1;
pub const MCA_ADDR: u8 = 2;
pub const MCA_MISC: u8 = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum McaMsr {
    Ctl,
    Status,
    Addr,
    Misc,
}

pub const fn mci_status_cec(status: u64) -> u64 {
    (status & MCI_STATUS_CEC_MASK) >> MCI_STATUS_CEC_SHIFT
}

pub const fn mci_status_mscod(status: u64) -> u64 {
    (status >> 16) & 0xffff
}

pub const fn mci_misc_addr_lsb(misc: u64) -> u8 {
    (misc & 0x3f) as u8
}

pub const fn mci_misc_addr_mode(misc: u64) -> u8 {
    ((misc >> 6) & 7) as u8
}

pub const fn xec(status: u64, mask: u64) -> u64 {
    (status >> 16) & mask
}

pub const fn mcg_ext_count(cap: u64) -> u64 {
    (cap & MCG_EXT_CNT_MASK) >> MCG_EXT_CNT_SHIFT
}

pub const fn mce_bank_count(cap: u64) -> usize {
    let banks = (cap & MCG_BANKCNT_MASK) as usize;
    if banks > MAX_NR_BANKS {
        MAX_NR_BANKS
    } else {
        banks
    }
}

pub const fn mca_msr_reg(bank: usize, reg: McaMsr, smca: bool) -> Result<u32, i32> {
    if bank >= MAX_NR_BANKS {
        return Err(EINVAL);
    }
    let bank = bank as u32;
    if smca {
        let base = match reg {
            McaMsr::Ctl => MSR_AMD64_SMCA_MC0_CTL,
            McaMsr::Status => MSR_AMD64_SMCA_MC0_STATUS,
            McaMsr::Addr => MSR_AMD64_SMCA_MC0_ADDR,
            McaMsr::Misc => MSR_AMD64_SMCA_MC0_MISC0,
        };
        Ok(base + 0x10 * bank)
    } else {
        let base = match reg {
            McaMsr::Ctl => MSR_IA32_MC0_CTL,
            McaMsr::Status => MSR_IA32_MC0_STATUS,
            McaMsr::Addr => MSR_IA32_MC0_ADDR,
            McaMsr::Misc => MSR_IA32_MC0_MISC,
        };
        Ok(base + 4 * bank)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Mce {
    pub status: u64,
    pub misc: u64,
    pub addr: u64,
    pub mcgstatus: u64,
    pub ip: u64,
    pub tsc: u64,
    pub time: u64,
    pub cpuvendor: CpuVendor,
    pub inject_flags: u8,
    pub severity: u8,
    pub pad: u8,
    pub cpuid: u32,
    pub cs: u8,
    pub bank: u8,
    pub cpu: u8,
    pub finished: u8,
    pub extcpu: u32,
    pub socketid: u32,
    pub apicid: u32,
    pub mcgcap: u64,
    pub synd: u64,
    pub ipid: u64,
    pub ppin: u64,
    pub microcode: u32,
    pub kflags: u64,
}

impl Default for Mce {
    fn default() -> Self {
        Self {
            status: 0,
            misc: 0,
            addr: 0,
            mcgstatus: 0,
            ip: 0,
            tsc: 0,
            time: 0,
            cpuvendor: CpuVendor::Unknown([0; 12]),
            inject_flags: 0,
            severity: 0,
            pad: 0,
            cpuid: 0,
            cs: 0,
            bank: 0,
            cpu: 0,
            finished: 0,
            extcpu: 0,
            socketid: 0,
            apicid: 0,
            mcgcap: 0,
            synd: 0,
            ipid: 0,
            ppin: 0,
            microcode: 0,
            kflags: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MceVendorAmd {
    pub synd1: u64,
    pub synd2: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MceHwErr {
    pub m: Mce,
    pub amd: MceVendorAmd,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct McaConfig {
    pub lmce_disabled: bool,
    pub disabled: bool,
    pub ser: bool,
    pub recovery: bool,
    pub bios_cmci_threshold: bool,
    pub initialized: bool,
    pub dont_log_ce: bool,
    pub cmci_disabled: bool,
    pub ignore_ce: bool,
    pub print_all: bool,
    pub monarch_timeout: i32,
    pub panic_timeout: i32,
    pub rip_msr: u32,
    pub bootlog: i8,
}

impl Default for McaConfig {
    fn default() -> Self {
        Self {
            lmce_disabled: false,
            disabled: false,
            ser: false,
            recovery: false,
            bios_cmci_threshold: false,
            initialized: false,
            dont_log_ce: false,
            cmci_disabled: false,
            ignore_ce: false,
            print_all: false,
            monarch_timeout: -1,
            panic_timeout: 0,
            rip_msr: 0,
            bootlog: -1,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MceVendorFlags {
    pub overflow_recov: bool,
    pub succor: bool,
    pub smca: bool,
    pub zen_ifu_quirk: bool,
    pub amd_threshold: bool,
    pub skx_repmov_quirk: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MceBank {
    pub ctl: u64,
    pub init: bool,
    pub lsb_in_status: bool,
}

impl Default for MceBank {
    fn default() -> Self {
        Self {
            ctl: u64::MAX,
            init: true,
            lsb_in_status: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MceBankSet {
    bits: u64,
}

impl MceBankSet {
    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    pub const fn all() -> Self {
        Self { bits: u64::MAX }
    }

    pub const fn from_bits(bits: u64) -> Self {
        Self { bits }
    }

    pub const fn bits(self) -> u64 {
        self.bits
    }

    pub fn set(&mut self, bank: usize) -> Result<(), i32> {
        if bank >= MAX_NR_BANKS {
            return Err(EINVAL);
        }
        self.bits |= 1u64 << bank;
        Ok(())
    }

    pub fn clear(&mut self, bank: usize) -> Result<(), i32> {
        if bank >= MAX_NR_BANKS {
            return Err(EINVAL);
        }
        self.bits &= !(1u64 << bank);
        Ok(())
    }

    pub const fn contains(self, bank: usize) -> bool {
        bank < MAX_NR_BANKS && (self.bits & (1u64 << bank)) != 0
    }

    pub const fn is_empty(self) -> bool {
        self.bits == 0
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct McpFlags {
    pub timestamp: bool,
    pub log_uncorrected: bool,
    pub queue_log: bool,
}

pub trait MsrAccess {
    fn read_msr(&self, msr: u32) -> Result<u64, i32>;
    fn write_msr(&mut self, msr: u32, value: u64) -> Result<(), i32>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct UnsupportedMsr;

impl MsrAccess for UnsupportedMsr {
    fn read_msr(&self, _msr: u32) -> Result<u64, i32> {
        Err(ENODEV)
    }

    fn write_msr(&mut self, _msr: u32, _value: u64) -> Result<(), i32> {
        Err(ENODEV)
    }
}

pub trait MceRecordSource {
    fn cpu_vendor(&self) -> CpuVendor;
    fn cpuid_eax1(&self) -> u32;
    fn mcg_cap(&self) -> u64;
    fn now_seconds(&self) -> u64;
    fn cpu(&self) -> u32;
    fn apicid(&self) -> u32;
    fn socketid(&self) -> u32;
    fn microcode(&self) -> u32;
    fn ppin(&self) -> u64;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StaticRecordSource {
    pub vendor: CpuVendor,
    pub cpuid: u32,
    pub mcg_cap: u64,
    pub now: u64,
    pub cpu: u32,
    pub apicid: u32,
    pub socketid: u32,
    pub microcode: u32,
    pub ppin: u64,
}

impl Default for StaticRecordSource {
    fn default() -> Self {
        Self {
            vendor: CpuVendor::Intel,
            cpuid: 0,
            mcg_cap: 0,
            now: 0,
            cpu: 0,
            apicid: 0,
            socketid: 0,
            microcode: 0,
            ppin: 0,
        }
    }
}

impl MceRecordSource for StaticRecordSource {
    fn cpu_vendor(&self) -> CpuVendor {
        self.vendor
    }

    fn cpuid_eax1(&self) -> u32 {
        self.cpuid
    }

    fn mcg_cap(&self) -> u64 {
        self.mcg_cap
    }

    fn now_seconds(&self) -> u64 {
        self.now
    }

    fn cpu(&self) -> u32 {
        self.cpu
    }

    fn apicid(&self) -> u32 {
        self.apicid
    }

    fn socketid(&self) -> u32 {
        self.socketid
    }

    fn microcode(&self) -> u32 {
        self.microcode
    }

    fn ppin(&self) -> u64 {
        self.ppin
    }
}

pub trait MceEventSink {
    fn push_mce(&mut self, err: MceHwErr) -> Result<(), i32>;
}

pub fn mce_prep_record_common<S: MceRecordSource>(source: &S, m: &mut Mce) {
    m.cpuid = source.cpuid_eax1();
    m.cpuvendor = source.cpu_vendor();
    m.mcgcap = source.mcg_cap();
    m.time = source.now_seconds();
}

pub fn mce_prep_record_per_cpu<S: MceRecordSource>(source: &S, m: &mut Mce) {
    let cpu = source.cpu();
    m.cpu = cpu as u8;
    m.extcpu = cpu;
    m.apicid = source.apicid();
    m.microcode = source.microcode();
    m.ppin = source.ppin();
    m.socketid = source.socketid();
}

pub fn mce_prep_record<S: MceRecordSource>(source: &S, err: &mut MceHwErr) {
    *err = MceHwErr::default();
    mce_prep_record_common(source, &mut err.m);
    mce_prep_record_per_cpu(source, &mut err.m);
}

pub fn mce_log<S: MceEventSink>(sink: &mut S, err: MceHwErr) -> Result<(), i32> {
    sink.push_mce(err)
}

pub const fn mce_available(features: CpuFeatures, cfg: McaConfig) -> bool {
    !cfg.disabled && features.has_mce() && features.has_mca()
}

pub fn mce_is_correctable(m: &Mce) -> bool {
    if matches!(m.cpuvendor, CpuVendor::Amd | CpuVendor::Hygon)
        && (m.status & MCI_STATUS_DEFERRED) != 0
    {
        return false;
    }
    (m.status & MCI_STATUS_UC) == 0
}

pub fn mce_is_memory_error(m: &Mce, flags: MceVendorFlags, bank: amd::AmdBankInfo) -> bool {
    match m.cpuvendor {
        CpuVendor::Amd | CpuVendor::Hygon => amd::amd_mce_is_memory_error(m, flags, bank.bank_type),
        CpuVendor::Intel | CpuVendor::Zhaoxin => {
            (m.status & 0xef80) == (1 << 7)
                || (m.status & 0xef00) == (1 << 8)
                || (m.status & 0xeffc) == 0x0c
        }
        _ => false,
    }
}

pub fn mce_usable_address(m: &Mce, flags: MceVendorFlags, bank: amd::AmdBankInfo) -> bool {
    if (m.status & MCI_STATUS_ADDRV) == 0 {
        return false;
    }
    match m.cpuvendor {
        CpuVendor::Amd | CpuVendor::Hygon => amd::amd_mce_usable_address(m, flags, bank),
        CpuVendor::Intel | CpuVendor::Zhaoxin => intel::intel_mce_usable_address(m),
        _ => true,
    }
}

pub fn machine_check_poll<A, S, Q>(
    flags: McpFlags,
    banks: MceBankSet,
    smca: bool,
    access: &mut A,
    source: &S,
    sink: &mut Q,
) -> Result<usize, i32>
where
    A: MsrAccess,
    S: MceRecordSource,
    Q: MceEventSink,
{
    let mut logged = 0;
    for bank in 0..MAX_NR_BANKS {
        if !banks.contains(bank) {
            continue;
        }
        let status = access.read_msr(mca_msr_reg(bank, McaMsr::Status, smca)?)?;
        if (status & MCI_STATUS_VAL) == 0 {
            continue;
        }

        let mut err = MceHwErr::default();
        mce_prep_record(source, &mut err);
        err.m.bank = bank as u8;
        err.m.status = status;
        err.m.finished = 1;
        if flags.timestamp {
            err.m.time = source.now_seconds();
        }
        if (status & MCI_STATUS_ADDRV) != 0 {
            err.m.addr = access.read_msr(mca_msr_reg(bank, McaMsr::Addr, smca)?)?;
        }
        if (status & MCI_STATUS_MISCV) != 0 {
            err.m.misc = access.read_msr(mca_msr_reg(bank, McaMsr::Misc, smca)?)?;
        }
        sink.push_mce(err)?;
        logged += 1;

        access.write_msr(mca_msr_reg(bank, McaMsr::Status, smca)?, 0)?;
    }
    Ok(logged)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::collections::BTreeMap;
    use alloc::vec::Vec;

    #[derive(Default)]
    struct MapMsr {
        regs: BTreeMap<u32, u64>,
        writes: Vec<(u32, u64)>,
    }

    impl MsrAccess for MapMsr {
        fn read_msr(&self, msr: u32) -> Result<u64, i32> {
            Ok(*self.regs.get(&msr).unwrap_or(&0))
        }

        fn write_msr(&mut self, msr: u32, value: u64) -> Result<(), i32> {
            self.writes.push((msr, value));
            self.regs.insert(msr, value);
            Ok(())
        }
    }

    #[derive(Default)]
    struct Sink(Vec<MceHwErr>);

    impl MceEventSink for Sink {
        fn push_mce(&mut self, err: MceHwErr) -> Result<(), i32> {
            self.0.push(err);
            Ok(())
        }
    }

    #[test]
    fn mce_bit_masks_match_linux_positions() {
        assert_eq!(MCI_STATUS_VAL, 1u64 << 63);
        assert_eq!(MCI_STATUS_UC, 1u64 << 61);
        assert_eq!(MCI_STATUS_ADDRV, 1u64 << 58);
        assert_eq!(MCI_STATUS_CEC_MASK, 0x7fff_u64 << 38);
        assert_eq!(mci_status_cec(9u64 << 38), 9);
        assert_eq!(MCG_STATUS_MCIP, 4);
    }

    #[test]
    fn mca_msr_numbering_selects_legacy_or_smca_ranges() {
        assert_eq!(mca_msr_reg(2, McaMsr::Status, false), Ok(0x409));
        assert_eq!(mca_msr_reg(2, McaMsr::Status, true), Ok(0xc000_2021));
        assert_eq!(
            mca_msr_reg(MAX_NR_BANKS, McaMsr::Status, false),
            Err(EINVAL)
        );
    }

    #[test]
    fn correctable_and_memory_error_classification_follow_vendor_rules() {
        let intel = Mce {
            cpuvendor: CpuVendor::Intel,
            status: MCI_STATUS_VAL | (1 << 7),
            ..Mce::default()
        };
        assert!(mce_is_correctable(&intel));
        assert!(mce_is_memory_error(
            &intel,
            MceVendorFlags::default(),
            amd::AmdBankInfo::default()
        ));

        let amd = Mce {
            cpuvendor: CpuVendor::Amd,
            status: MCI_STATUS_VAL | MCI_STATUS_DEFERRED,
            ..Mce::default()
        };
        assert!(!mce_is_correctable(&amd));
    }

    #[test]
    fn machine_check_poll_logs_valid_banks_and_clears_status() {
        let mut msr = MapMsr::default();
        msr.regs.insert(
            mca_msr_reg(1, McaMsr::Status, false).unwrap(),
            MCI_STATUS_VAL | MCI_STATUS_ADDRV | MCI_STATUS_MISCV,
        );
        msr.regs
            .insert(mca_msr_reg(1, McaMsr::Addr, false).unwrap(), 0xfeed_0000);
        msr.regs
            .insert(mca_msr_reg(1, McaMsr::Misc, false).unwrap(), 0x80);
        let mut banks = MceBankSet::empty();
        banks.set(1).unwrap();
        let source = StaticRecordSource {
            vendor: CpuVendor::Intel,
            now: 77,
            mcg_cap: 4,
            ..StaticRecordSource::default()
        };
        let mut sink = Sink::default();

        assert_eq!(
            machine_check_poll(
                McpFlags {
                    timestamp: true,
                    log_uncorrected: false,
                    queue_log: true,
                },
                banks,
                false,
                &mut msr,
                &source,
                &mut sink
            ),
            Ok(1)
        );
        assert_eq!(sink.0[0].m.bank, 1);
        assert_eq!(sink.0[0].m.addr, 0xfeed_0000);
        assert!(
            msr.writes
                .contains(&(mca_msr_reg(1, McaMsr::Status, false).unwrap(), 0))
        );
    }

    #[test]
    fn unsupported_msr_fails_closed() {
        let mut unsupported = UnsupportedMsr;
        let mut sink = Sink::default();
        assert_eq!(
            machine_check_poll(
                McpFlags::default(),
                MceBankSet::all(),
                false,
                &mut unsupported,
                &StaticRecordSource::default(),
                &mut sink
            ),
            Err(ENODEV)
        );
    }
}
