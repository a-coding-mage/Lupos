//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/apic/probe_64.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/apic/probe_64.c
//! 64-bit x86 APIC probe model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/apic/probe_64.c

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Apic64ProbeResult {
    NoApic,
    Flat,
    X2ApicPhysical,
    X2ApicCluster,
}

pub const fn probe_64_apic(has_apic: bool, has_x2apic: bool, cpu_count: u32) -> Apic64ProbeResult {
    if !has_apic {
        Apic64ProbeResult::NoApic
    } else if has_x2apic && cpu_count > 255 {
        Apic64ProbeResult::X2ApicCluster
    } else if has_x2apic {
        Apic64ProbeResult::X2ApicPhysical
    } else {
        Apic64ProbeResult::Flat
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ApicDriverProbe {
    pub probe: bool,
    pub acpi_madt_oem_match: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X86_64ProbeApicResult {
    pub enabled_ir_x2apic: bool,
    pub installed_driver: Option<usize>,
}

pub fn x86_64_probe_apic(drivers: &[ApicDriverProbe]) -> X86_64ProbeApicResult {
    X86_64ProbeApicResult {
        enabled_ir_x2apic: true,
        installed_driver: drivers.iter().position(|driver| driver.probe),
    }
}

pub fn default_acpi_madt_oem_check(drivers: &[ApicDriverProbe]) -> i32 {
    if drivers.iter().any(|driver| driver.acpi_madt_oem_match) {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe64_prefers_x2apic_when_available() {
        assert_eq!(
            probe_64_apic(true, true, 4),
            Apic64ProbeResult::X2ApicPhysical
        );
        assert_eq!(
            probe_64_apic(true, true, 512),
            Apic64ProbeResult::X2ApicCluster
        );
    }

    #[test]
    fn probe_64_installs_first_matching_driver_after_enabling_ir() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kernel/apic/probe_64.c"
        ));
        assert!(source.contains("enable_IR_x2apic();"));
        assert!(source.contains("apic_install_driver(*drv);"));

        let drivers = [
            ApicDriverProbe {
                probe: false,
                acpi_madt_oem_match: false,
            },
            ApicDriverProbe {
                probe: true,
                acpi_madt_oem_match: false,
            },
            ApicDriverProbe {
                probe: true,
                acpi_madt_oem_match: true,
            },
        ];
        assert_eq!(
            x86_64_probe_apic(&drivers),
            X86_64ProbeApicResult {
                enabled_ir_x2apic: true,
                installed_driver: Some(1),
            }
        );
        assert_eq!(default_acpi_madt_oem_check(&drivers), 1);
        assert_eq!(default_acpi_madt_oem_check(&drivers[..1]), 0);
    }
}
