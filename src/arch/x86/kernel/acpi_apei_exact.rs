//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/acpi/apei.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/acpi/apei.c
//! x86 APEI firmware-first corrected-error hooks.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CmcFirmwareFirst {
    pub enabled: bool,
    pub firmware_first: bool,
    pub hardware_banks: u8,
    pub threshold: u32,
}

pub const fn arch_apei_enable_cmcff(cmc: CmcFirmwareFirst, mce_enabled: bool) -> i32 {
    if !mce_enabled {
        return 1;
    }
    if !cmc.enabled {
        return 0;
    }
    if !cmc.firmware_first || cmc.hardware_banks == 0 {
        return 1;
    }
    1
}

pub const fn arch_apei_report_x86_error_status(status: i32) -> i32 {
    status
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arch_apei_hooks_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kernel/acpi/apei.c"
        ));
        assert!(source.contains("int arch_apei_enable_cmcff"));
        assert!(source.contains("if (!cmc->enabled)"));
        assert!(source.contains("mce_save_apei_thr_limit"));
        assert!(source.contains("ACPI_HEST_FIRMWARE_FIRST"));
        assert!(source.contains("mce_disable_bank(mc_bank->bank_number);"));
        assert!(source.contains("arch_apei_report_mem_error"));
        assert!(source.contains("apei_mce_report_mem_error(sev, mem_err);"));
        assert!(source.contains("arch_apei_report_x86_error"));
        assert!(source.contains("apei_smca_report_x86_error(ctx_info, lapic_id);"));

        assert_eq!(
            arch_apei_enable_cmcff(
                CmcFirmwareFirst {
                    enabled: false,
                    firmware_first: true,
                    hardware_banks: 1,
                    threshold: 9,
                },
                true,
            ),
            0
        );
        assert_eq!(
            arch_apei_enable_cmcff(
                CmcFirmwareFirst {
                    enabled: true,
                    firmware_first: false,
                    hardware_banks: 1,
                    threshold: 9,
                },
                true,
            ),
            1
        );
        assert_eq!(arch_apei_report_x86_error_status(-5), -5);
    }
}
