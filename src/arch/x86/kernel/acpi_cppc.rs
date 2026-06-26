//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! x86 ACPI CPPC helpers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/acpi/cppc.c

use crate::include::uapi::errno::EINVAL;

pub const CPPC_HIGHEST_PERF_PERFORMANCE: u64 = 196;
pub const CPPC_HIGHEST_PERF_PREFCORE: u64 = 166;
pub const MSR_AMD_CPPC_CAP1: u64 = 0xc001_02b0;
pub const AMD_CPPC_HIGHEST_PERF_MASK: u64 = 0xff;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X86Vendor {
    Amd,
    Hygon,
    Intel,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CpuInfo {
    pub vendor: X86Vendor,
    pub family: u8,
    pub model: u8,
    pub has_cppc: bool,
    pub has_aperfmperf: bool,
    pub has_zen4: bool,
    pub has_amd_htr_cores: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TopologyCpuType {
    Unknown,
    Performance,
    Efficiency,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CpcReg {
    pub address: u64,
    pub bit_width: u8,
    pub bit_offset: u8,
}

pub trait CppcMsr {
    fn read_msr_on_cpu(&mut self, cpu: usize, msr: u64) -> Result<u64, i32>;
    fn write_msr_on_cpu(&mut self, cpu: usize, msr: u64, val: u64) -> Result<(), i32>;
}

pub const fn cpc_supported_by_cpu(cpu: CpuInfo) -> bool {
    match cpu.vendor {
        X86Vendor::Amd | X86Vendor::Hygon => {
            if cpu.family == 0x19 && (cpu.model <= 0x0f || (cpu.model >= 0x20 && cpu.model <= 0x2f))
            {
                true
            } else if cpu.family == 0x17 && cpu.model >= 0x30 && cpu.model <= 0x7f {
                true
            } else {
                cpu.has_cppc
            }
        }
        _ => false,
    }
}

pub const fn cpc_ffh_supported() -> bool {
    true
}

pub const fn genmask_u64(high: u8, low: u8) -> u64 {
    if high >= 64 || low > high {
        0
    } else {
        let width = high - low + 1;
        if width == 64 {
            u64::MAX
        } else {
            ((1u64 << width) - 1) << low
        }
    }
}

pub fn cpc_read_ffh<M: CppcMsr>(msr: &mut M, cpunum: usize, reg: CpcReg) -> Result<u64, i32> {
    if reg.bit_width == 0 || reg.bit_offset >= 64 {
        return Err(EINVAL);
    }
    let high = reg.bit_offset + reg.bit_width - 1;
    let mask = genmask_u64(high, reg.bit_offset);
    if mask == 0 {
        return Err(EINVAL);
    }
    let val = msr.read_msr_on_cpu(cpunum, reg.address)?;
    Ok((val & mask) >> reg.bit_offset)
}

pub fn cpc_write_ffh<M: CppcMsr>(
    msr: &mut M,
    cpunum: usize,
    reg: CpcReg,
    val: u64,
) -> Result<u64, i32> {
    if reg.bit_width == 0 || reg.bit_offset >= 64 {
        return Err(EINVAL);
    }
    let high = reg.bit_offset + reg.bit_width - 1;
    let mask = genmask_u64(high, reg.bit_offset);
    if mask == 0 {
        return Err(EINVAL);
    }
    let mut rd_val = msr.read_msr_on_cpu(cpunum, reg.address)?;
    rd_val &= !mask;
    rd_val |= (val << reg.bit_offset) & mask;
    msr.write_msr_on_cpu(cpunum, reg.address, rd_val)?;
    Ok(rd_val)
}

pub fn amd_detect_prefcore(highest_perfs: &[u32]) -> Result<(bool, u64), i32> {
    if highest_perfs.is_empty() {
        return Err(EINVAL);
    }
    let first = highest_perfs[0];
    let detected = highest_perfs.iter().any(|&v| v != first);
    Ok((detected, first as u64))
}

pub fn amd_get_boost_ratio_numerator(
    cpu: CpuInfo,
    core_type: TopologyCpuType,
    prefcore: bool,
    highest_perf: u64,
) -> Result<u64, i32> {
    if !prefcore {
        return Ok(highest_perf);
    }
    if cpu.has_zen4 && cpu.model >= 0x70 && cpu.model <= 0x7f {
        return Ok(CPPC_HIGHEST_PERF_PERFORMANCE);
    }
    if cpu.has_amd_htr_cores {
        return match core_type {
            TopologyCpuType::Performance => Ok(CPPC_HIGHEST_PERF_PERFORMANCE),
            TopologyCpuType::Efficiency => Ok(highest_perf),
            TopologyCpuType::Unknown => Err(EINVAL),
        };
    }
    Ok(CPPC_HIGHEST_PERF_PREFCORE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct MockMsr {
        value: u64,
        written: u64,
    }

    impl CppcMsr for MockMsr {
        fn read_msr_on_cpu(&mut self, _cpu: usize, _msr: u64) -> Result<u64, i32> {
            Ok(self.value)
        }
        fn write_msr_on_cpu(&mut self, _cpu: usize, _msr: u64, val: u64) -> Result<(), i32> {
            self.written = val;
            self.value = val;
            Ok(())
        }
    }

    #[test]
    fn cpc_supported_matches_amd_family_model_rules() {
        assert!(cpc_supported_by_cpu(CpuInfo {
            vendor: X86Vendor::Amd,
            family: 0x19,
            model: 0x20,
            has_cppc: false,
            has_aperfmperf: false,
            has_zen4: false,
            has_amd_htr_cores: false,
        }));
        assert!(!cpc_supported_by_cpu(CpuInfo {
            vendor: X86Vendor::Intel,
            family: 0,
            model: 0,
            has_cppc: true,
            has_aperfmperf: true,
            has_zen4: false,
            has_amd_htr_cores: false,
        }));
    }

    #[test]
    fn ffh_read_write_apply_bit_offset_and_width() {
        let reg = CpcReg {
            address: MSR_AMD_CPPC_CAP1,
            bit_width: 8,
            bit_offset: 8,
        };
        let mut msr = MockMsr {
            value: 0xab00,
            written: 0,
        };
        assert_eq!(cpc_read_ffh(&mut msr, 0, reg), Ok(0xab));
        assert_eq!(cpc_write_ffh(&mut msr, 0, reg, 0x12), Ok(0x1200));
        assert_eq!(msr.written, 0x1200);
    }

    #[test]
    fn prefcore_detection_and_boost_numerator_follow_amd_rules() {
        assert_eq!(amd_detect_prefcore(&[100, 101]).unwrap(), (true, 100));
        let cpu = CpuInfo {
            vendor: X86Vendor::Amd,
            family: 0x19,
            model: 0x75,
            has_cppc: true,
            has_aperfmperf: true,
            has_zen4: true,
            has_amd_htr_cores: false,
        };
        assert_eq!(
            amd_get_boost_ratio_numerator(cpu, TopologyCpuType::Unknown, true, 100),
            Ok(CPPC_HIGHEST_PERF_PERFORMANCE)
        );
    }
}
