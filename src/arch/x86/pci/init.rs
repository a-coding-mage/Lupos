//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/pci/init.c
//! test-origin: linux:vendor/linux/arch/x86/pci/init.c
//! x86 PCI architecture initcall sequencing.

pub const PCI_PROBE_NOEARLY: u32 = 1 << 10;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PciArchInitInput {
    pub pci_probe: u32,
    pub direct_probe_type: i32,
    pub arch_init_result: Option<i32>,
    pub raw_pci_ops: bool,
    pub raw_pci_ext_ops: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PciArchInitTrace {
    pub direct_probe_called: bool,
    pub mmcfg_early_init_called: bool,
    pub arch_init_called: bool,
    pub msi_domain_created: bool,
    pub pcbios_init_called: bool,
    pub direct_init_type: Option<i32>,
    pub fatal_no_config_space: bool,
    pub dmi_pciprobe_checked: bool,
    pub dmi_skip_isa_align_checked: bool,
}

pub const fn pci_arch_init_sequence(input: PciArchInitInput) -> PciArchInitTrace {
    let mut trace = PciArchInitTrace {
        direct_probe_called: true,
        ..PciArchInitTrace::empty()
    };

    if input.pci_probe & PCI_PROBE_NOEARLY == 0 {
        trace.mmcfg_early_init_called = true;
    }

    let mut pcbios = 1;
    if let Some(result) = input.arch_init_result {
        trace.arch_init_called = true;
        pcbios = result;
    }

    trace.msi_domain_created = true;
    if pcbios == 0 {
        return trace;
    }

    trace.pcbios_init_called = true;
    trace.direct_init_type = Some(input.direct_probe_type);
    trace.fatal_no_config_space = !input.raw_pci_ops && !input.raw_pci_ext_ops;
    trace.dmi_pciprobe_checked = true;
    trace.dmi_skip_isa_align_checked = true;
    trace
}

impl PciArchInitTrace {
    pub const fn empty() -> Self {
        Self {
            direct_probe_called: false,
            mmcfg_early_init_called: false,
            arch_init_called: false,
            msi_domain_created: false,
            pcbios_init_called: false,
            direct_init_type: None,
            fatal_no_config_space: false,
            dmi_pciprobe_checked: false,
            dmi_skip_isa_align_checked: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pci_arch_init_sequence_matches_linux_source_order() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/pci/init.c"
        ));
        assert!(source.contains("type = pci_direct_probe();"));
        assert!(source.contains("if (!(pci_probe & PCI_PROBE_NOEARLY))"));
        assert!(source.contains("pci_mmcfg_early_init();"));
        assert!(source.contains("x86_init.pci.arch_init()"));
        assert!(source.contains("x86_create_pci_msi_domain();"));
        assert!(source.contains("if (!pcbios)"));
        assert!(source.contains("pci_pcbios_init();"));
        assert!(source.contains("pci_direct_init(type);"));
        assert!(source.contains("No config space access function found"));
        assert!(source.contains("dmi_check_pciprobe();"));
        assert!(source.contains("arch_initcall(pci_arch_init);"));

        let full = pci_arch_init_sequence(PciArchInitInput {
            pci_probe: 0,
            direct_probe_type: 7,
            arch_init_result: Some(1),
            raw_pci_ops: false,
            raw_pci_ext_ops: false,
        });
        assert!(full.direct_probe_called);
        assert!(full.mmcfg_early_init_called);
        assert_eq!(full.direct_init_type, Some(7));
        assert!(full.fatal_no_config_space);

        let no_pcbios = pci_arch_init_sequence(PciArchInitInput {
            pci_probe: PCI_PROBE_NOEARLY,
            direct_probe_type: 3,
            arch_init_result: Some(0),
            raw_pci_ops: true,
            raw_pci_ext_ops: false,
        });
        assert!(!no_pcbios.mmcfg_early_init_called);
        assert!(no_pcbios.msi_domain_created);
        assert!(!no_pcbios.pcbios_init_called);
    }
}
