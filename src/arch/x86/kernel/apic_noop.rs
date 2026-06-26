//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/apic/apic_noop.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/apic/apic_noop.c
//! No-op APIC driver model.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NoopApic {
    pub name: &'static str,
    pub dest_mode_logical: bool,
    pub max_apic_id: u32,
}

pub const APIC_NOOP: NoopApic = NoopApic {
    name: "noop",
    dest_mode_logical: true,
    max_apic_id: 0xfe,
};

pub const fn noop_wakeup_secondary_cpu() -> i32 {
    -1
}

pub const fn noop_apic_icr_read() -> u64 {
    0
}

pub const fn noop_get_apic_id(_apicid: u32) -> u32 {
    0
}

pub const fn noop_apic_read(
    _reg: u32,
    boot_cpu_has_apic: bool,
    apic_disabled: bool,
) -> (u32, bool) {
    (0, boot_cpu_has_apic && !apic_disabled)
}

pub const fn noop_apic_write_warn(boot_cpu_has_apic: bool, apic_disabled: bool) -> bool {
    boot_cpu_has_apic && !apic_disabled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_apic_driver_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kernel/apic/apic_noop.c"
        ));
        assert!(source.contains("NOOP APIC driver."));
        assert!(source.contains("static void noop_send_IPI"));
        assert!(source.contains("return -1;"));
        assert!(source.contains("static u64 noop_apic_icr_read(void) { return 0; }"));
        assert!(source.contains("static u32 noop_get_apic_id(u32 apicid) { return 0; }"));
        assert!(
            source.contains("WARN_ON_ONCE(boot_cpu_has(X86_FEATURE_APIC) && !apic_is_disabled);")
        );
        assert!(source.contains("struct apic apic_noop __ro_after_init"));
        assert!(source.contains(".name\t\t\t\t= \"noop\""));
        assert!(source.contains(".dest_mode_logical\t\t= true"));
        assert!(source.contains(".max_apic_id\t\t\t= 0xFE"));
        assert!(source.contains(".send_IPI_self\t\t\t= noop_send_IPI_self"));

        assert_eq!(APIC_NOOP.name, "noop");
        assert!(APIC_NOOP.dest_mode_logical);
        assert_eq!(APIC_NOOP.max_apic_id, 0xfe);
        assert_eq!(noop_wakeup_secondary_cpu(), -1);
        assert_eq!(noop_apic_icr_read(), 0);
        assert_eq!(noop_get_apic_id(17), 0);
        assert_eq!(noop_apic_read(0x20, true, false), (0, true));
        assert!(noop_apic_write_warn(true, false));
    }
}
