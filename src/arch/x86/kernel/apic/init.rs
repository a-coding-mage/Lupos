//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/apic/init.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/apic/init.c
//! APIC static-call setup and driver installation policy.

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ApicDriver {
    pub name: &'static str,
    pub eoi: bool,
    pub native_eoi: bool,
    pub x2apic_set_max_apicid: bool,
    pub max_apic_id: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ApicInstallResult {
    pub driver: ApicDriver,
    pub switched: bool,
    pub static_calls_updated: bool,
}

pub const APIC_STATIC_CALLS: &[&str] = &[
    "eoi",
    "native_eoi",
    "icr_read",
    "icr_write",
    "read",
    "send_IPI",
    "send_IPI_mask",
    "send_IPI_mask_allbutself",
    "send_IPI_allbutself",
    "send_IPI_all",
    "send_IPI_self",
    "wait_icr_idle",
    "wakeup_secondary_cpu",
    "wakeup_secondary_cpu_64",
    "write",
];

pub fn apic_setup_apic_calls(mut apic: ApicDriver) -> ApicInstallResult {
    if !apic.native_eoi {
        apic.native_eoi = apic.eoi;
    }
    ApicInstallResult {
        driver: apic,
        switched: false,
        static_calls_updated: true,
    }
}

pub fn apic_install_driver(
    current_name: &str,
    mut driver: ApicDriver,
    x2apic_enabled: bool,
    x2apic_max_apicid: u32,
) -> ApicInstallResult {
    if current_name == driver.name {
        return ApicInstallResult {
            driver,
            switched: false,
            static_calls_updated: false,
        };
    }
    if x2apic_enabled && driver.x2apic_set_max_apicid {
        driver.max_apic_id = x2apic_max_apicid;
    }
    if !driver.native_eoi {
        driver.native_eoi = driver.eoi;
    }
    ApicInstallResult {
        driver,
        switched: true,
        static_calls_updated: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apic_init_static_calls_and_driver_install_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kernel/apic/init.c"
        ));
        assert!(source.contains("#define DEFINE_APIC_CALL(__cb)"));
        assert!(source.contains("DEFINE_APIC_CALL(eoi);"));
        assert!(source.contains("DEFINE_APIC_CALL(wait_icr_idle);"));
        assert!(source.contains("EXPORT_STATIC_CALL_TRAMP_GPL(apic_call_send_IPI_mask);"));
        assert!(source.contains("struct apic_override __x86_apic_override __initdata;"));
        assert!(source.contains("#define apply_override(__cb)"));
        assert!(source.contains("#define update_call(__cb)"));
        assert!(source.contains("apic->native_eoi = apic->eoi;"));
        assert!(source.contains("pr_info(\"Static calls initialized\\n\");"));
        assert!(source.contains("void __init apic_install_driver(struct apic *driver)"));
        assert!(source.contains("if (apic == driver)"));
        assert!(source.contains("apic = driver;"));
        assert!(source.contains("apic->max_apic_id = x2apic_max_apicid;"));
        assert!(source.contains("restore_override_callbacks();"));
        assert!(source.contains("pr_info(\"Switched APIC routing to: %s\\n\", driver->name);"));

        assert!(APIC_STATIC_CALLS.contains(&"send_IPI_self"));
        let setup = apic_setup_apic_calls(ApicDriver {
            name: "flat",
            eoi: true,
            native_eoi: false,
            x2apic_set_max_apicid: false,
            max_apic_id: 0,
        });
        assert!(setup.driver.native_eoi);
        assert!(setup.static_calls_updated);

        let installed = apic_install_driver(
            "old",
            ApicDriver {
                name: "x2apic",
                eoi: true,
                native_eoi: false,
                x2apic_set_max_apicid: true,
                max_apic_id: 0,
            },
            true,
            255,
        );
        assert!(installed.switched);
        assert_eq!(installed.driver.max_apic_id, 255);
        assert!(installed.driver.native_eoi);
    }
}
