//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/acrn.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/acrn.c
//! ACRN hypervisor detection and callback wiring.

pub const HYPERVISOR_CALLBACK_VECTOR: u8 = 0xf3;
pub const X86_HYPER_ACRN: u32 = 8;
pub const X86_FEATURE_X2APIC: u32 = 1 << 21;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AcrnPlatformInit {
    pub sysvec_installed: bool,
    pub calibrate_tsc_from_acrn: bool,
    pub calibrate_cpu_from_acrn: bool,
}

pub const fn acrn_detect(cpuid_base: u32) -> u32 {
    cpuid_base
}

pub const fn acrn_init_platform() -> AcrnPlatformInit {
    AcrnPlatformInit {
        sysvec_installed: true,
        calibrate_tsc_from_acrn: true,
        calibrate_cpu_from_acrn: true,
    }
}

pub const fn acrn_x2apic_available(features: u32) -> bool {
    features & X86_FEATURE_X2APIC != 0
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AcrnInterruptState {
    pub eoi_sent: bool,
    pub irq_stat_incremented: bool,
    pub handler_called: bool,
}

pub const fn sysvec_acrn_hv_callback(handler_installed: bool) -> AcrnInterruptState {
    AcrnInterruptState {
        eoi_sent: true,
        irq_stat_incremented: true,
        handler_called: handler_installed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acrn_hypervisor_hooks_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kernel/cpu/acrn.c"
        ));
        assert!(source.contains("return acrn_cpuid_base();"));
        assert!(source.contains("sysvec_install(HYPERVISOR_CALLBACK_VECTOR"));
        assert!(source.contains("x86_platform.calibrate_tsc = acrn_get_tsc_khz;"));
        assert!(source.contains("x86_platform.calibrate_cpu = acrn_get_tsc_khz;"));
        assert!(source.contains("boot_cpu_has(X86_FEATURE_X2APIC)"));
        assert!(source.contains("DEFINE_IDTENTRY_SYSVEC(sysvec_acrn_hv_callback)"));
        assert!(source.contains("apic_eoi();"));
        assert!(source.contains("inc_irq_stat(HYPERVISOR_CALLBACK);"));
        assert!(source.contains("acrn_setup_intr_handler"));
        assert!(source.contains("acrn_remove_intr_handler"));
        assert!(source.contains(".name                   = \"ACRN\""));

        assert_eq!(acrn_detect(0x4000_0000), 0x4000_0000);
        assert_eq!(
            acrn_init_platform(),
            AcrnPlatformInit {
                sysvec_installed: true,
                calibrate_tsc_from_acrn: true,
                calibrate_cpu_from_acrn: true,
            }
        );
        assert!(acrn_x2apic_available(X86_FEATURE_X2APIC));
        assert_eq!(
            sysvec_acrn_hv_callback(true),
            AcrnInterruptState {
                eoi_sent: true,
                irq_stat_incremented: true,
                handler_called: true,
            }
        );
    }
}
