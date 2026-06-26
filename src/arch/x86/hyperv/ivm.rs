//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/hyperv/ivm.c
//! test-origin: linux:vendor/linux/arch/x86/hyperv/ivm.c
//! Hyper-V isolated VM model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/hyperv/ivm.c

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HvIsolationType {
    None,
    Vbs,
    SevSnp,
    Tdx,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HvIsolationConfig {
    pub isolation_type: HvIsolationType,
    pub shared_gpa_boundary: u64,
}

pub const fn isolation_from_cpuid(ebx: u32, shared_boundary: u64) -> HvIsolationConfig {
    let isolation_type = match ebx & 0x0f {
        1 => HvIsolationType::Vbs,
        2 => HvIsolationType::SevSnp,
        3 => HvIsolationType::Tdx,
        _ => HvIsolationType::None,
    };
    HvIsolationConfig {
        isolation_type,
        shared_gpa_boundary: shared_boundary,
    }
}

pub const fn isolation_active(config: HvIsolationConfig) -> bool {
    !matches!(config.isolation_type, HvIsolationType::None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isolation_low_bits_select_type() {
        let cfg = isolation_from_cpuid(2, 1 << 47);
        assert_eq!(cfg.isolation_type, HvIsolationType::SevSnp);
        assert!(isolation_active(cfg));
    }
}
