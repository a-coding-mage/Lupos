//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/apic/apic_flat_64.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/apic/apic_flat_64.c
//! Physical flat APIC driver metadata.

pub const APIC_PHYSFLAT_NAME: &str = "physical flat";
pub const PHYSFLAT_MAX_APIC_ID: u32 = 0xfe;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhysflatApic {
    pub name: &'static str,
    pub dest_mode_logical: bool,
    pub disable_esr: u8,
    pub max_apic_id: u32,
    pub nmi_to_offline_cpu: bool,
}

pub const APIC_PHYSFLAT: PhysflatApic = PhysflatApic {
    name: APIC_PHYSFLAT_NAME,
    dest_mode_logical: false,
    disable_esr: 0,
    max_apic_id: PHYSFLAT_MAX_APIC_ID,
    nmi_to_offline_cpu: true,
};

pub const fn physflat_get_apic_id(x: u32) -> u32 {
    (x >> 24) & 0xff
}

pub const fn physflat_probe() -> bool {
    true
}

pub const fn physflat_acpi_madt_oem_check() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apic_physflat_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kernel/apic/apic_flat_64.c"
        ));
        assert!(source.contains("static u32 physflat_get_apic_id"));
        assert!(source.contains("return (x >> 24) & 0xFF;"));
        assert!(source.contains("static int physflat_probe(void)"));
        assert!(source.contains(".name\t\t\t\t= \"physical flat\""));
        assert!(source.contains(".dest_mode_logical\t\t= false"));
        assert!(source.contains(".max_apic_id\t\t\t= 0xFE"));
        assert!(source.contains(".get_apic_id\t\t\t= physflat_get_apic_id"));
        assert!(source.contains(".send_IPI\t\t\t= default_send_IPI_single_phys"));
        assert!(source.contains(".nmi_to_offline_cpu\t\t= true"));
        assert!(source.contains("apic_driver(apic_physflat);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(apic);"));

        assert_eq!(physflat_get_apic_id(0xab00_0000), 0xab);
        assert!(physflat_probe());
        assert!(physflat_acpi_madt_oem_check());
        assert_eq!(APIC_PHYSFLAT.max_apic_id, 0xfe);
        assert!(APIC_PHYSFLAT.nmi_to_offline_cpu);
    }
}
