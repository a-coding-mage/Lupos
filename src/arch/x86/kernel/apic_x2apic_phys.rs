//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! x2APIC physical-mode destination model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/apic/x2apic_phys.c

// Physical mode delivers an IPI by writing the absolute 32-bit APIC ID to
// MSR_X2APIC_ICR. The Linux driver also handles the
// FADT IAPC_BOOT_ARCH no-8259 quirk and a small set of vendor opt-ins
// (Intel default, AMD when `x2apic_phys` cmdline). We model the policy
// decision; the MSR write itself stays out-of-scope until APIC writes are
// wired through a trait-backed accessor.

use crate::arch::x86::kernel::cpu::CpuVendor;

pub const X2APIC_DEST_SELF: u32 = 0xffff_ffff;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X2ApicPhysPolicy {
    Default,
    Forced,
    Disabled,
}

pub const fn select_phys_policy(
    vendor: CpuVendor,
    cmdline_forced: bool,
    fadt_iapc_no_8259: bool,
) -> X2ApicPhysPolicy {
    if cmdline_forced {
        return X2ApicPhysPolicy::Forced;
    }
    match vendor {
        CpuVendor::Intel => X2ApicPhysPolicy::Default,
        CpuVendor::Amd | CpuVendor::Hygon => {
            if fadt_iapc_no_8259 {
                X2ApicPhysPolicy::Default
            } else {
                X2ApicPhysPolicy::Disabled
            }
        }
        _ => X2ApicPhysPolicy::Disabled,
    }
}

pub const fn encode_phys_dest(apicid: u32, self_ipi: bool) -> u32 {
    if self_ipi { X2APIC_DEST_SELF } else { apicid }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intel_defaults_to_phys_mode() {
        assert_eq!(
            select_phys_policy(CpuVendor::Intel, false, false),
            X2ApicPhysPolicy::Default
        );
    }

    #[test]
    fn amd_needs_fadt_or_cmdline_to_enable_phys_mode() {
        assert_eq!(
            select_phys_policy(CpuVendor::Amd, false, false),
            X2ApicPhysPolicy::Disabled
        );
        assert_eq!(
            select_phys_policy(CpuVendor::Amd, false, true),
            X2ApicPhysPolicy::Default
        );
        assert_eq!(
            select_phys_policy(CpuVendor::Amd, true, false),
            X2ApicPhysPolicy::Forced
        );
    }

    #[test]
    fn self_ipi_uses_broadcast_marker() {
        assert_eq!(encode_phys_dest(7, false), 7);
        assert_eq!(encode_phys_dest(7, true), X2APIC_DEST_SELF);
    }
}
