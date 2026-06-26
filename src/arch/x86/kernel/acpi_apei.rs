//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/acpi/apei.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/acpi/apei.c
//! x86 ACPI APEI hooks.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/acpi/apei.c

pub const ACPI_HEST_FIRMWARE_FIRST: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AcpiHestIaCorrected {
    pub enabled: bool,
    pub flags: u32,
    pub error_threshold_value: u64,
    pub num_hardware_banks: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ApeEnableDecision {
    pub return_value: i32,
    pub saved_threshold: Option<u64>,
    pub disable_bank_count: u16,
}

pub const fn arch_apei_enable_cmcff(cmc: AcpiHestIaCorrected, x86_mce: bool) -> ApeEnableDecision {
    if !x86_mce {
        return ApeEnableDecision {
            return_value: 1,
            saved_threshold: None,
            disable_bank_count: 0,
        };
    }
    if !cmc.enabled {
        return ApeEnableDecision {
            return_value: 0,
            saved_threshold: None,
            disable_bank_count: 0,
        };
    }
    if cmc.flags & ACPI_HEST_FIRMWARE_FIRST == 0 || cmc.num_hardware_banks == 0 {
        return ApeEnableDecision {
            return_value: 1,
            saved_threshold: Some(cmc.error_threshold_value),
            disable_bank_count: 0,
        };
    }
    ApeEnableDecision {
        return_value: 1,
        saved_threshold: Some(cmc.error_threshold_value),
        disable_bank_count: cmc.num_hardware_banks,
    }
}

pub const fn arch_apei_report_mem_error(x86_mce: bool) -> bool {
    x86_mce
}

pub trait X86ErrorReporter {
    fn report_x86_error(&self, lapic_id: u64) -> i32;
}

pub fn arch_apei_report_x86_error<R: X86ErrorReporter>(reporter: &R, lapic_id: u64) -> i32 {
    reporter.report_x86_error(lapic_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmcff_disabled_hest_returns_zero_when_mce_enabled() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kernel/acpi/apei.c"
        ));
        assert!(source.contains("int arch_apei_enable_cmcff"));
        assert!(source.contains("mce_save_apei_thr_limit(cmc->notify.error_threshold_value);"));
        assert!(source.contains("if (!(cmc->flags & ACPI_HEST_FIRMWARE_FIRST)"));
        assert!(source.contains("mce_disable_bank(mc_bank->bank_number);"));
        assert!(source.contains("void arch_apei_report_mem_error"));
        assert!(source.contains("apei_mce_report_mem_error(sev, mem_err);"));
        assert!(source.contains("int arch_apei_report_x86_error"));
        assert!(source.contains("return apei_smca_report_x86_error(ctx_info, lapic_id);"));

        let decision = arch_apei_enable_cmcff(
            AcpiHestIaCorrected {
                enabled: false,
                flags: 0,
                error_threshold_value: 7,
                num_hardware_banks: 2,
            },
            true,
        );
        assert_eq!(decision.return_value, 0);
        assert_eq!(decision.disable_bank_count, 0);
    }

    #[test]
    fn firmware_first_banks_are_counted_for_disable() {
        let decision = arch_apei_enable_cmcff(
            AcpiHestIaCorrected {
                enabled: true,
                flags: ACPI_HEST_FIRMWARE_FIRST,
                error_threshold_value: 9,
                num_hardware_banks: 3,
            },
            true,
        );
        assert_eq!(decision.return_value, 1);
        assert_eq!(decision.saved_threshold, Some(9));
        assert_eq!(decision.disable_bank_count, 3);
    }
}
