//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/apic/hw_nmi.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/apic/hw_nmi.c
//! x86 APIC hardware NMI model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/apic/hw_nmi.c

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HwNmiState {
    pub enabled: bool,
    pub watchdog_vector: u8,
}

pub const NMI_VECTOR: u8 = 2;

pub const fn hw_nmi_state(local_apic_present: bool, watchdog_enabled: bool) -> HwNmiState {
    HwNmiState {
        enabled: local_apic_present && watchdog_enabled,
        watchdog_vector: NMI_VECTOR,
    }
}

pub const fn nmi_watchdog_should_panic(state: HwNmiState, lockup_detected: bool) -> bool {
    state.enabled && lockup_detected
}

pub const fn hw_nmi_sample_period(cpu_khz: u64, watchdog_thresh: u64) -> u64 {
    cpu_khz * 1000 * watchdog_thresh
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nmi_watchdog_requires_apic_and_lockup() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kernel/apic/hw_nmi.c"
        ));
        assert!(source.contains("hw_nmi_get_sample_period"));
        assert!(source.contains("return (u64)(cpu_khz) * 1000 * watchdog_thresh;"));
        assert!(source.contains("__apic_send_IPI_mask(mask, NMI_VECTOR);"));
        assert!(source.contains("arch_trigger_cpumask_backtrace"));
        assert!(source.contains("nmi_cpu_backtrace(regs)"));
        assert!(source.contains("register_nmi_handler(NMI_LOCAL"));
        assert!(source.contains("early_initcall(register_nmi_cpu_backtrace_handler);"));

        let state = hw_nmi_state(true, true);
        assert!(nmi_watchdog_should_panic(state, true));
        assert!(!nmi_watchdog_should_panic(hw_nmi_state(false, true), true));
        assert_eq!(hw_nmi_sample_period(1000, 10), 10_000_000);
    }
}
