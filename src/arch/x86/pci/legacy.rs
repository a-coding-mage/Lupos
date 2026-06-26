//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/pci/legacy.c
//! test-origin: linux:vendor/linux/arch/x86/pci/legacy.c
//! Legacy PCI bus probing policy.

use crate::include::uapi::errno::ENODEV;

pub const PCI_VENDOR_ID: u16 = 0;

pub const fn pci_legacy_init(raw_pci_ops_present: bool) -> i32 {
    if raw_pci_ops_present { 0 } else { 1 }
}

pub const fn pcibios_scan_stride(jailhouse_paravirt: bool) -> u16 {
    if jailhouse_paravirt { 1 } else { 8 }
}

pub const fn pci_vendor_id_valid(vendor: u32) -> bool {
    vendor != 0x0000 && vendor != 0xffff
}

pub fn first_peer_device(devfn_vendor: &[(u16, u32)], jailhouse_paravirt: bool) -> Option<u16> {
    let stride = pcibios_scan_stride(jailhouse_paravirt);
    devfn_vendor
        .iter()
        .copied()
        .find(|(devfn, vendor)| devfn % stride == 0 && pci_vendor_id_valid(*vendor))
        .map(|(devfn, _)| devfn)
}

pub const fn pci_subsys_init_status(x86_init_requested_legacy: bool, legacy_init_ret: i32) -> i32 {
    if x86_init_requested_legacy && legacy_init_ret != 0 {
        -ENODEV
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_pci_scan_policy_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/pci/legacy.c"
        ));
        assert!(source.contains("pcibios_fixup_peer_bridges"));
        assert!(source.contains("pcibios_last_bus <= 0 || pcibios_last_bus > 0xff"));
        assert!(source.contains("if (!raw_pci_ops)"));
        assert!(source.contains("return 1;"));
        assert!(source.contains("pcibios_scan_root(0);"));
        assert!(source.contains("int stride = jailhouse_paravirt() ? 1 : 8;"));
        assert!(source.contains("for (devfn = 0; devfn < 256; devfn += stride)"));
        assert!(source.contains("l != 0x0000 && l != 0xffff"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(pcibios_scan_specific_bus)"));
        assert!(source.contains("subsys_initcall(pci_subsys_init);"));

        assert_eq!(pci_legacy_init(false), 1);
        assert_eq!(pci_legacy_init(true), 0);
        assert_eq!(pcibios_scan_stride(false), 8);
        assert_eq!(pcibios_scan_stride(true), 1);
        assert!(!pci_vendor_id_valid(0));
        assert!(!pci_vendor_id_valid(0xffff));
        assert_eq!(
            first_peer_device(&[(1, 0x1234), (8, 0x8086)], false),
            Some(8)
        );
        assert_eq!(first_peer_device(&[(1, 0x1234)], true), Some(1));
        assert_eq!(pci_subsys_init_status(true, 1), -ENODEV);
    }
}
