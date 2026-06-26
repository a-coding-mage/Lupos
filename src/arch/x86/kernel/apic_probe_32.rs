//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! 32-bit x86 APIC probe model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/apic/probe_32.c

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Apic32ProbeResult {
    NoApic,
    Bigsmp,
    Default,
}

pub const fn probe_32_apic(has_apic: bool, cpu_count: u32) -> Apic32ProbeResult {
    if !has_apic {
        Apic32ProbeResult::NoApic
    } else if cpu_count > 8 {
        Apic32ProbeResult::Bigsmp
    } else {
        Apic32ProbeResult::Default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe32_promotes_large_systems_to_bigsmp() {
        assert_eq!(probe_32_apic(false, 1), Apic32ProbeResult::NoApic);
        assert_eq!(probe_32_apic(true, 16), Apic32ProbeResult::Bigsmp);
    }
}
